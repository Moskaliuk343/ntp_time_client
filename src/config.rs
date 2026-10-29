
use std::env;

pub struct Config {
    pub server: String,
    pub count: u32,
    pub interval: u64,
}

impl Config {
    pub fn from_args() -> Result<Option<Self>, String> {
        let args: Vec<String> = env::args().skip(1).collect();
        Self::parse_args(&args)
    }

    fn parse_args(args: &[String]) -> Result<Option<Self>, String> {
        let mut config = Config {
            server: "pool.ntp.org".to_string(),
            count: 1,
            interval: 2,
        };

        let mut i = 0;

        while i < args.len() {
            match args[i].as_str() {
                "--help" | "-h" => {
                    Self::print_help();
                    return Ok(None);
                }

                "--server" => {
                    i += 1;
                    let value = args.get(i).ok_or(
                        "Не вказано адресу після --server"
                    )?;

                    if value.is_empty() || value.starts_with('-') {
                        return Err("Некоректна адреса NTP-сервера".into());
                    }

                    config.server = value.clone();
                }

                "--count" => {
                    i += 1;
                    let value = args.get(i).ok_or(
                        "Не вказано число після --count"
                    )?;

                    config.count = value.parse().map_err(|_| {
                        "Кількість запитів повинна бути числом"
                    })?;

                    if !(1..=100).contains(&config.count) {
                        return Err(
                            "--count повинен бути від 1 до 100".into()
                        );
                    }
                }

                "--interval" => {
                    i += 1;
                    let value = args.get(i).ok_or(
                        "Не вказано число після --interval"
                    )?;

                    config.interval = value.parse().map_err(|_| {
                        "Інтервал повинен бути цілим числом"
                    })?;

                    if !(1..=60).contains(&config.interval) {
                        return Err(
                            "--interval повинен бути від 1 до 60".into()
                        );
                    }
                }

                other => {
                    return Err(format!(
                        "Невідомий параметр: {}. Використай --help",
                        other
                    ));
                }
            }

            i += 1;
        }

        Ok(Some(config))
    }

    fn print_help() {
        println!("NTP Time Client v0.3.0");
        println!();
        println!("Використання:");
        println!("  cargo run -- [параметри]");
        println!();
        println!("Параметри:");
        println!("  --server HOST   Адреса NTP-сервера");
        println!("  --count N       Кількість запитів (1-100)");
        println!("  --interval N    Інтервал у секундах (1-60)");
        println!("  --help, -h      Показати довідку");
        println!();
        println!("Значення за замовчуванням:");
        println!("  Сервер: pool.ntp.org");
        println!("  Кількість: 1");
        println!("  Інтервал: 2 сек.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration() {
        let config = Config::parse_args(&[])
            .unwrap()
            .unwrap();

        assert_eq!(config.server, "pool.ntp.org");
        assert_eq!(config.count, 1);
        assert_eq!(config.interval, 2);
    }

    #[test]
    fn custom_configuration() {
        let args = vec![
            "--server".to_string(),
            "time.google.com".to_string(),
            "--count".to_string(),
            "5".to_string(),
            "--interval".to_string(),
            "3".to_string(),
        ];

        let config = Config::parse_args(&args)
            .unwrap()
            .unwrap();

        assert_eq!(config.server, "time.google.com");
        assert_eq!(config.count, 5);
        assert_eq!(config.interval, 3);
    }

    #[test]
    fn invalid_count() {
        let args = vec![
            "--count".to_string(),
            "0".to_string(),
        ];

        assert!(Config::parse_args(&args).is_err());
    }

    #[test]
    fn unknown_argument() {
        let args = vec!["--unknown".to_string()];

        assert!(Config::parse_args(&args).is_err());
    }
}
