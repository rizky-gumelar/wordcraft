# Panduan Build WordCraft untuk Android (Tablet)

Aplikasi WordCraft Android (`apps/wordcraft-android`) mengemas antarmuka WordCraft berbasis `eframe` (egui) ke dalam aplikasi native Android (`android-activity`) yang dioptimalkan untuk perangkat tablet.

---

## 1. Prasyarat (Prerequisites)

Untuk meng-compile WordCraft menjadi file `.apk`, pastikan Anda telah menginstal perangkat berikut pada komputer pengembangan Anda:

### A. Rust Toolchain & Target Android
Jika belum memiliki Rust, instal melalui [rustup.rs](https://rustup.rs):
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Tambahkan target arsitektur Android (kebanyakan tablet modern menggunakan `aarch64`):
```sh
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
```

### B. Android SDK & NDK
Pastikan Anda memiliki Android SDK dan NDK (disarankan NDK versi 25 ke atas).
Atur *environment variable* ke direktori SDK dan NDK Anda, contoh:
```sh
# macOS Homebrew setup:
export ANDROID_HOME="/opt/homebrew/share/android-commandlinetools"
export NDK_HOME="/opt/homebrew/share/android-ndk"
```

### C. Tool Pengemas `cargo-apk`
Instal tool resmi pengemas APK untuk Rust:
```sh
cargo install cargo-apk
```

---

## 2. Cara Build APK

Dari direktori root proyek `wordcraft`:

### Build APK Siap Pakai (Release):
```sh
cargo apk build --release -p wordcraft-android
```
Hasil APK yang sudah di-sign dan di-align berada di:
`target/release/apk/wordcraft_android.apk`


---

## 3. Cara Install & Menjalankan di Tablet

Sambungkan tablet Android Anda via kabel USB (atau ADB over Wi-Fi), pastikan **USB Debugging** aktif pada Pengaturan Developer tablet Anda.

### Pasang langsung via cargo-apk:
```sh
cargo apk run --release -p wordcraft-android
```

### Atau pasang manual menggunakan ADB:
```sh
adb install -r target/release/apk/wordcraft-android.apk
```

---

## 4. Fitur Khusus di Tablet Android

1. **Font Sistem Terintegrasi**:
   Aplikasi otomatis memindai direktori `/system/fonts` Android (termasuk font Roboto, Noto Serif, dan Noto Color Emoji) di samping font bawaan WordCraft.
2. **File Browser Internal**:
   WordCraft langsung menyediakan browser file layar sentuh untuk membuka dan menyimpan dokumen (.docx, .odt, .rtf, .txt, .pdf) dari `/sdcard` atau folder `Documents` tablet.
3. **Dukungan Aksesori Tablet**:
   Mendukung penuh navigasi sentuh, Stylus / Pen, serta Keyboard Cover dan Mouse Bluetooth untuk pengalaman pengetikan ala desktop.
