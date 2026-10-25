# Panduan Lengkap Porting Aplikasi Rust Desktop (egui/winit) ke Android

Dokumen ini berisi rangkuman teknis, pengalaman nyata, dan *best practices* dalam mem-porting aplikasi Rust desktop (khususnya yang menggunakan GUI framework seperti `egui`, `eframe`, atau `winit`) menjadi aplikasi Android native (.APK) yang siap pakai. Panduan ini dirancang sebagai referensi untuk pengembangan aplikasi Rust lainnya di masa depan.

---

## 1. Perbedaan Konseptual: Desktop vs Android

| Aspek | Desktop (macOS / Linux / Windows) | Android |
|---|---|---|
| **Eksekusi** | Binary executable mandiri (`fn main()`) | Shared library (`cdylib` `.so`) di dalam paket `.apk` |
| **Lifecycle** | Dikontrol langsung oleh OS / loop window | Dikontrol oleh Activity Lifecycle (`onCreate`, `onResume`, `onDestroy`) |
| **Entrypoint** | `fn main()` | `#[unsafe(no_mangle)] fn android_main(app: AndroidApp)` |
| **Driver GPU** | Vulkan, DirectX 12, Metal, OpenGL | OpenGL ES (Glow) dan Vulkan |
| **Input** | Mouse (pointer presisi, klik kanan, scroll wheel) | Layar sentuh (jari, multi-touch, gestur, virtual keyboard/IME) |
| **Font Sistem** | `/Library/Fonts`, `C:\Windows\Fonts`, `/usr/share/fonts` | `/system/fonts` (Roboto, Noto, dsb.) |

---

## 2. Struktur Runner Android (`apps/<app-name>-android`)

Untuk menjaga codebase inti tetap bersih, buatlah crate runner khusus Android di dalam workspace:

```
apps/my-app-android/
├── Cargo.toml
├── AndroidManifest.xml
├── res/                           # Folder aset Android
│   ├── mipmap-mdpi/ic_launcher.png    (48x48)
│   ├── mipmap-hdpi/ic_launcher.png    (72x72)
│   ├── mipmap-xhdpi/ic_launcher.png   (96x96)
│   ├── mipmap-xxhdpi/ic_launcher.png  (144x144)
│   └── mipmap-xxxhdpi/ic_launcher.png (192x192)
├── src/
│   └── lib.rs                     # Entrypoint android_main
└── my-app.keystore                 # Keystore untuk signing APK
```

### Konfigurasi `Cargo.toml`
```toml
[package]
name = "my-app-android"
version = "0.1.0"
edition = "2024"

# Izinkan unsafe untuk #[unsafe(no_mangle)] pada entrypoint Android
[lints.rust]
unsafe_code = "allow"

[lib]
crate-type = ["cdylib"]
name = "my_app_android"

[dependencies]
# Dependensi core aplikasi Anda
my-app-core = { path = "../../crates/core" }

# eframe & winit untuk Android
eframe = { version = "0.36", default-features = false, features = ["default_fonts", "glow", "android-native-activity"] }
egui = "0.36"
winit = { version = "0.30", default-features = false }
log = "0.4"
android_logger = "0.14"

[package.metadata.android]
package = "com.example.myapp"
build_targets = ["aarch64-linux-android", "x86_64-linux-android"]
resources = "res"

[package.metadata.android.application]
label = "My App"
icon = "@mipmap/ic_launcher"
theme = "@android:style/Theme.NoTitleBar.Fullscreen"
has_code = false

[package.metadata.android.activity]
name = "android.app.NativeActivity"

[package.metadata.android.signing.release]
path = "my-app.keystore"
keystore_password = "password123"
key_alias = "my_app"
key_password = "password123"
```

---

## 3. Catatan Kritis & Solusi Bug Umum

### A. Jebakan `GameActivity` vs `NativeActivity` (Penyebab Force Close Pertama)
- **Gejala**: Aplikasi langsung *force close* saat pertama kali di-tap di tablet/HP.
- **Penyebab**: Menggunakan `com.google.androidgamesdk.GameActivity` tanpa menyertakan runtime Java `.dex`. Android OS mencari class Java tersebut dan menghasilkan `ClassNotFoundException`.
- **Solusi**: Jika aplikasi Anda adalah aplikasi Rust murni (`has_code = false`), **wajib menggunakan `android.app.NativeActivity`** dan fitur `android-native-activity` pada `eframe`/`winit`. `NativeActivity` sudah terpasang langsung di dalam OS Android.

### B. Kompatibilitas GPU: MediaTek / Mali vs Qualcomm Adreno
- **Gejala**: Crash layar hitam atau driver error saat membuat surface layar sentuh.
- **Penyebab**: Chipset MediaTek Helio (seperti Helio G99, G100, G200 dengan GPU Mali) sering memiliki implementasi driver Vulkan yang kurang lengkap atau *buggy* saat berinteraksi dengan windowing native.
- **Solusi**: Gunakan backend **OpenGL ES (`glow`)** daripada Vulkan (`wgpu`):
  ```rust
  options.renderer = eframe::Renderer::Glow;
  ```
  OpenGL ES adalah standar grafis utama sistem UI Android, sehingga 100% stabil di semua merk SoC (MediaTek, Snapdragon, Exynos, Unisoc).

### C. Masalah Keyboard Virtual (IME) Selalu Muncul
- **Gejala**: Setiap kali user menggeser layar (scroll) atau memencet tombol menu/toolbar, keyboard HP/tablet selalu muncul mengganggu.
- **Penyebab**:
  1. Pada desktop, aplikasi sering mengaktifkan auto-refocus ke area editor saat tombol toolbar ditekan agar user bisa lanjut mengetik dengan keyboard fisik. Di Android, ini memerintahkan sistem memunculkan software keyboard.
  2. Event `drag_started` (saat swipe/scroll) sering dianggap sebagai interaksi yang meminta fokus.
- **Solusi**:
  1. **Matikan auto-refocus chrome untuk Android**:
     ```rust
     #[cfg(not(target_os = "android"))]
     self.canvas.want_focus = true;
     ```
  2. **Bedakan ketukan pada kertas vs luar kertas**:
     ```rust
     #[cfg(target_os = "android")]
     let on_paper = resp.interact_pointer_pos().is_some_and(|p| rects.iter().any(|pr| pr.contains(p)));
     
     // Hanya minta fokus keyboard jika user benar-benar mengetuk kertas dokumen
     #[cfg(target_os = "android")]
     let focus_click = pointer_click && on_paper;
     
     // Jika user swipe/scroll atau ketuk background abu-abu, sembunyikan keyboard
     #[cfg(target_os = "android")]
     if (pointer_click && !on_paper) || resp.drag_started() {
         resp.surrender_focus();
     }
     ```

### D. Optimasi Gestur Layar Sentuh (Touch UX)
1. **Pinch to Zoom (Dua Jari)**:
   Gunakan API `multi_touch()` egui untuk membaca perubahan jarak dua jari:
   ```rust
   if let Some(multi) = ui.input(|i| i.multi_touch()) {
       if (multi.zoom_delta - 1.0).abs() > 1e-3 {
           let new_zoom = current_zoom * multi.zoom_delta;
           apply_zoom(new_zoom);
       }
   }
   ```
2. **Pemisahan Gestur Swipe (Di Dalam vs Di Luar Kertas Dokumen)**:
   Agar geseran di luar kertas dokumen tidak secara tidak sengaja menyeleksi kata/teks:
   - **Di dalam kertas (`touch_started_on_paper = true`)**: Sentuhan dan geseran fokus untuk menyeleksi teks/kata (`caret.set` dengan `extend: true` atau `select.word`). Canvas tidak di-scroll secara paksa.
   - **Di luar kertas (`touch_started_on_paper = false`)**: Geseran digunakan murni untuk memindahkan/menggulir viewport (`scroll_offset -= drag_delta`), dan fungsi seleksi teks di-bypass total:
   ```rust
   // Saat pertama kali jari menekan layar:
   if ui.input(|i| i.pointer.primary_pressed()) {
       let p = ui.input(|i| i.pointer.press_origin().or_else(|| i.pointer.latest_pos()));
       let on_paper = p.is_some_and(|pos| page_rects.iter().any(|r| r.contains(pos)));
       touch_started_on_paper = Some(on_paper);
   }

   // Scroll hanya aktif jika sentuhan dimulai di luar kertas:
   let touch_delta = if touch_started_on_paper == Some(false) && ui.input(|i| i.pointer.primary_down()) {
       -ui.input(|i| i.pointer.delta())
   } else {
       Vec2::ZERO
   };

   // Di handler mouse/text:
   if touch_started_on_paper == Some(false) {
       return; // Jangan seleksi kata atau pindahkan kursor
   }
   ```
3. **Memperbesar Target Sentuh (Touch Targets)**:
   Jari manusia jauh lebih besar daripada ujung kursor mouse (pixel). Perbesar radius interaksi tombol:
   ```rust
   let mut style = (*ctx.style()).clone();
   style.interaction.interact_radius = 8.0; // Memudahkan tombol kecil di-tap
   style.spacing.button_padding = egui::vec2(6.0, 5.0);
   ctx.set_style(style);
   ```

### E. Optimasi Keyboard Fisik (Hardware Keyboard & Shortcut Mirip Word Windows)
Saat tablet dipasangkan keyboard fisik (Bluetooth, USB OTG, atau Keyboard Cover/Dock):
1. **Mencegah Activity Restart saat Keyboard Dipasang/Dilepas**:
   Tambahkan `keyboard|navigation` pada `configChanges` di `AndroidManifest.xml`:
   ```xml
   android:configChanges="orientation|keyboard|keyboardHidden|navigation|screenSize|screenLayout|smallestScreenSize"
   ```
2. **Klaim Fokus Otomatis saat Tombol Fisik Ditekan**:
   Pada layar sentuh, fokus sengaja dilepas saat scroll di luar kertas agar keyboard virtual tidak muncul. Namun saat keyboard fisik ditekan, aplikasi harus langsung merespons shortcut / ketikan tanpa memaksa user mengetuk kertas terlebih dahulu:
   ```rust
   let has_key_events = ui.input(|i| {
       i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. } | egui::Event::Text(_)))
   });
   if (app.canvas.focused || (has_key_events && app.dialog.is_none())) {
       if !app.canvas.focused && has_key_events {
           app.canvas.want_focus = true;
       }
       crate::keys::canvas_events(app, ui.ctx());
   }
   ```
3. **Mendukung Tombol Modifiers Windows (Ctrl) & Mac/Android (Cmd/Meta)**:
   Di Android, keyboard eksternal bisa memiliki tombol `Ctrl` (layout Windows/PC) maupun `Cmd/Meta/Search`. Petakan keduanya ke `Mod+` agar shortcut standar Word (`Ctrl+B`, `Ctrl+I`, `Ctrl+C`, `Ctrl+V`, `Ctrl+Z`, `Ctrl+S`, `Ctrl+A`, dll.) langsung berfungsi:
   ```rust
   let cmd = m.command || (cfg!(target_os = "android") && (m.ctrl || m.mac_cmd));
   if cmd {
       s.push_str("Mod+");
   }
   ```

---

## 4. Alur Build & Tooling

### Prasyarat di Mesin Pengembang:
1. **Rust Target**:
   ```sh
   rustup target add aarch64-linux-android x86_64-linux-android
   ```
2. **Android Commandline Tools & NDK**:
   - SDK: `platforms;android-30` (atau lebih baru) dan `build-tools`.
   - NDK: r25 ke atas (misal r30).
3. **Tool Pengemas**:
   ```sh
   cargo install cargo-apk
   ```

### Membuat Keystore untuk Signing:
```sh
keytool -genkeypair -v -keystore my-app.keystore -alias my_app \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -storepass password123 -keypass password123 \
  -dname "CN=My App, O=My Company, C=ID"
```

### Build Perintah Sekali Jalan:
```sh
export ANDROID_HOME="/opt/homebrew/share/android-commandlinetools"
export NDK_HOME="/opt/homebrew/share/android-ndk"

cargo apk build --release -p my-app-android
```
Hasil APK: `target/release/apk/my_app_android.apk`.

### Pasang ke Perangkat:
```sh
adb install -r target/release/apk/my_app_android.apk
```

---

## 5. Penanganan Font di Android (Calibri, Times New Roman, & Custom Fonts)

### Tantangan:
Android tidak memiliki font bawaan desktop seperti **Calibri**, **Times New Roman**, **Arial**, atau **Cambria**. Selain itu, file binary font asli Microsoft berpemilik (*proprietary*) dan tidak boleh di-bundle ke dalam open-source repo/APK tanpa lisensi komersial.

### Solusi Terbaik (Metric-Compatible OFL Fonts):
Gunakan font pengganti yang 100% kompatibel secara metrik (lebar glyph, spasi, dan tinggi baris identik sehingga layout dokumen tidak bergeser):
- **Calibri** $\rightarrow$ **Carlito** (SIL Open Font License)
- **Times New Roman** $\rightarrow$ **Tinos** / **Liberation Serif** (SIL Open Font License)
- **Arial** $\rightarrow$ **Liberation Sans** / **Arimo** (SIL Open Font License)

### Langkah Implementasi:
1. **Bundle Font Metrik di Rust**:
   Unduh font OFL ke `assets/fonts/` dan masukkan ke `BUNDLED`:
   ```rust
   static BUNDLED: &[&[u8]] = &[
       include_bytes!("../../../assets/fonts/Carlito-Regular.ttf"),
       include_bytes!("../../../assets/fonts/Carlito-Bold.ttf"),
       include_bytes!("../../../assets/fonts/Carlito-Italic.ttf"),
       include_bytes!("../../../assets/fonts/Carlito-BoldItalic.ttf"),
       include_bytes!("../../../assets/fonts/Tinos-Regular.ttf"),
       include_bytes!("../../../assets/fonts/Tinos-Bold.ttf"),
       include_bytes!("../../../assets/fonts/Tinos-Italic.ttf"),
       include_bytes!("../../../assets/fonts/Tinos-BoldItalic.ttf"),
       // ...
   ];
   ```
2. **Daftarkan Alias di Font Picker**:
   Agar pengguna langsung melihat pilihan "Calibri" dan "Times New Roman" di antarmuka aplikasi.
3. **Dukungan Direktori User Fonts**:
   Izinkan pengguna menaruh file font `.ttf`/`.otf` apa pun langsung di perangkat tablet (misal `/sdcard/Android/data/<package>/files/fonts` atau `/sdcard/Documents/<App>/fonts`):
   ```rust
   if let Some(mut path) = app.external_data_path() {
       path.push("fonts");
       let _ = std::fs::create_dir_all(&path);
       wordcraft_fonts::FontDb::global().load_user_fonts(&path);
   }
   ```
