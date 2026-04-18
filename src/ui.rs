use inquire::PasswordDisplayMode;

pub fn welcome_message() {
    println!();
    println!("============================================");
    println!("  Selamat datang di comma!");
    println!("  AI-powered git commit generator.");
    println!("============================================");
    println!();
    println!("Pertama-tama, kita perlu sedikit konfigurasi.");
    println!();
}

pub fn error_message(message: &str) {
    eprintln!();
    eprintln!("============================================");
    eprintln!("  ERROR");
    eprintln!("============================================");
    eprintln!();
    eprintln!("  {}", message);
    println!();
}

pub fn api_key_prompt() -> String {
    inquire::Password::new("Masukkan OpenRouter API Key (sk-or-v1-...):")
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_help_message("API key bisa diambil di https://openrouter.ai/keys")
        .prompt()
        .expect("User cancelled")
}

pub fn model_select_prompt(models: &[String]) -> String {
    use inquire::Select;

    let mut options: Vec<String> = vec!["[ Ketik Manual ID Model... ]".to_string()];
    options.extend(models.iter().cloned());

    let sel = Select::new("Pilih model (ketik untuk mencari):", options)
        .with_page_size(20)
        .prompt()
        .expect("User cancelled");

    sel
}

pub fn manual_model_prompt() -> String {
    inquire::Text::new("Masukkan ID model secara manual (contoh: anthropic/claude-3-haiku):")
        .prompt()
        .expect("User cancelled")
}

pub fn save_confirmation() {
    println!();
    println!("Konfigurasi disimpan!");
    println!();
}

pub fn fetching_models_message() {
    print!("Mengambil daftar model dari OpenRouter...");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}

pub fn models_loaded(count: usize) {
    println!(" done! {} model ditemukan.", count);
    println!("Ketik untuk mencari...");
    println!();
}

pub fn rate_limited_message() {
    error_message("Terlalu banyak permintaan. Mohon tunggu sebentar dan coba lagi.");
}

pub fn confirm_large_diff(size: usize) -> bool {
    println!();
    println!("⚠️ Diff terlalu besar ({} karakter).", size);
    println!();
    println!("Commit dengan diff sebanyak ini merupakan anti-pattern Git:");
    println!("- Kemungkinan me-stage file yang tidak seharusnya (lock files, dist/)");
    println!("- Sebaiknya dipecah menjadi commit yang lebih kecil per fitur");
    println!();
    inquire::Confirm::new("Tetap lanjut?")
        .with_default(false)
        .prompt()
        .unwrap_or(false)
}

pub fn print_unstaged_files(files: &[crate::preflight::UnstagedFile]) {
    println!("⚠️ Tidak ada file yang di-stage untuk di-commit.");
    println!();
    println!("File yang berubah tapi belum di-stage:");
    for file in files {
        println!(" {} {}", file.status, file.path);
    }
    println!();
}

pub fn prompt_git_add() -> bool {
    inquire::Confirm::new("Apakah kamu ingin melakukan 'git add .' sekarang?")
        .with_default(true)
        .prompt()
        .unwrap_or(false)
}
