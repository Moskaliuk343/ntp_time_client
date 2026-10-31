
mod config;
mod logger;
mod ntp;
mod ntp_packet;

use chrono::Local;
use config::Config;
use std::thread;
use std::time::Duration;

fn write_log_or_exit(message: &str) {
    if let Err(error) = logger::log_event(message) {
        eprintln!("Помилка логування: {}", error);
        std::process::exit(1);
    }
}

fn main() {
    println!("====================================");
    println!("       NTP Time Client v1.0.0");
    println!("====================================");

    let config = match Config::from_args() {
        Ok(Some(config)) => config,
        Ok(None) => return,
        Err(error) => {
            eprintln!("Помилка налаштувань: {}", error);
            std::process::exit(1);
        }
    };

    println!("ОС: {}", std::env::consts::OS);
    println!("NTP-сервер: {}", config.server);
    println!("Кількість запитів: {}", config.count);
    println!("Інтервал: {} сек.", config.interval);
    println!("Журнал: {}", logger::LOG_FILE);
    println!("------------------------------------");

    write_log_or_exit(&format!(
        "START | server={} | count={} | interval={}s",
        config.server, config.count, config.interval
    ));

    let mut successful = 0;
    let mut failed = 0;

    for number in 1..=config.count {
        println!("\n[Запит {} із {}]", number, config.count);

        match ntp::get_ntp_time(&config.server) {
            Ok(result) => {
                successful += 1;

                let local_time =
                    result.estimated_time.with_timezone(&Local);

                println!(
                    "Час сервера (T3): {}",
                    result.server_time.format(
                        "%Y-%m-%d %H:%M:%S%.3f"
                    )
                );

                println!(
                    "Оцінка часу UTC: {}",
                    result.estimated_time.format(
                        "%Y-%m-%d %H:%M:%S%.3f"
                    )
                );

                println!(
                    "Місцевий час: {}",
                    local_time.format(
                        "%Y-%m-%d %H:%M:%S%.3f %:z"
                    )
                );

                println!(
                    "Мережева затримка: {:.3} мс",
                    result.delay_ms
                );

                println!(
                    "Зміщення годинника: {:+.3} мс",
                    result.offset_ms
                );

                println!("Статус: Успішно");

                write_log_or_exit(&format!(
                    "SUCCESS | server={} | request={} | utc={} | delay_ms={:.3} | offset_ms={:+.3}",
                    config.server,
                    number,
                    result.estimated_time.format(
                        "%Y-%m-%d %H:%M:%S%.3f"
                    ),
                    result.delay_ms,
                    result.offset_ms
                ));
            }

            Err(error) => {
                failed += 1;

                eprintln!("Помилка NTP: {}", error);

                write_log_or_exit(&format!(
                    "ERROR | server={} | request={} | error={}",
                    config.server, number, error
                ));
            }
        }

        if number < config.count {
            thread::sleep(Duration::from_secs(config.interval));
        }
    }

    println!("\n====================================");
    println!("Успішних запитів: {}", successful);
    println!("Невдалих запитів: {}", failed);
    println!("Моніторинг завершено.");

    write_log_or_exit(&format!(
        "FINISH | successful={} | failed={}",
        successful, failed
    ));

    if failed > 0 {
        std::process::exit(1);
    }
}
