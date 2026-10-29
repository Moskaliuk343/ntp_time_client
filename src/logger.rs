
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub const LOG_FILE: &str = "logs/ntp.log";

// Запис одного повідомлення в журнал.
pub fn log_event(message: &str) -> Result<(), String> {
    let path = Path::new(LOG_FILE);
    append_log(path, message)
}

fn append_log(path: &Path, message: &str) -> Result<(), String> {
    // Створюємо папку журналу, якщо її ще немає.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Помилка створення папки: {}", e))?;
    }

    // Відкриваємо файл для дописування.
    // Старі записи не видаляються.
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Помилка відкриття журналу: {}", e))?;

    // Час запису за локальним годинником ПК.
    let timestamp = Local::now()
        .format("%Y-%m-%d %H:%M:%S%.3f %:z");

    writeln!(file, "{} | {}", timestamp, message)
        .map_err(|e| format!("Помилка запису в журнал: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_creates_file_and_appends() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let path = std::env::temp_dir().join(format!(
            "ntp_log_test_{}_{}.log",
            std::process::id(),
            unique
        ));

        append_log(&path, "Перший запис").unwrap();
        append_log(&path, "Другий запис").unwrap();

        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("Перший запис"));
        assert!(content.contains("Другий запис"));
        assert_eq!(content.lines().count(), 2);

        fs::remove_file(path).unwrap();
    }
}
