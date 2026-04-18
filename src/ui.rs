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
        .expect("Failed to read API key")
}

pub fn model_select_prompt(models: &[String]) -> String {
    use inquire::Select;

    let mut options: Vec<String> = vec!["[ Ketik Manual ID Model... ]".to_string()];
    options.extend(models.iter().cloned());

    let sel = Select::new(
        "Pilih model (ketik untuk mencari):",
        options,
    )
    .with_page_size(20)
    .prompt()
    .expect("Failed to select model");

    sel
}

pub fn manual_model_prompt() -> String {
    inquire::Text::new("Masukkan ID model secara manual (contoh: anthropic/claude-3-haiku):")
        .prompt()
        .expect("Failed to read manual model ID")
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
