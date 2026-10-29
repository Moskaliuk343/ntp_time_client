
mod config;
mod logger;
mod ntp;

use chrono::Local;
use config::Config;
use std::thread;
use std::time::Duration;

// Записуємо повідомлення в журнал.
// Якщо запис неможливий, завершуємо програму.
fn write_log_or_exit(message: &str) {
    if let Err(error) = logger::log_event(message) {
        eprintln!("Помилка логування: {}", error);
        std::process::exit(1);
    }
}

fn main() {
    println!("====================================");
    println!("       NTP Time Client v0.4.0");
    println!("====================================");

    let config = match Config::from_args() {
        Ok(Some(config)) => config,
        Ok(None) => return,
        Err(error) => {
            eprintln!("Помилка налаштувань: {}", error);
            std::process::exit(1);
        }
    };

    println!("Операційна система: {}", std::env::consts::OS);
    println!("NTP-сервер: {}", config.server);
    println!("Кількість запитів: {}", config.count);
    println!("Інтервал: {} сек.", config.interval);
    println!("Файл журналу: {}", logger::LOG_FILE);

    println!("------------------------------------");

    write_log_or_exit(&format!(
        "START | server={} | count={} | interval={}s",
        config.server, config.count, config.interval
    ));

    let mut successful = 0;
    let mut failed = 0;

    println!("Початок отримання NTP-часу...\n");

    for number in 1..=config.count {
        println!("[Запит {} із {}]", number, config.count);

        match ntp::get_ntp_time(&config.server) {
            Ok(utc_time) => {
                let local_time = utc_time.with_timezone(&Local);

                println!(
                    "UTC: {}",
                    utc_time.format("%Y-%m-%d %H:%M:%S%.3f")
                );

                println!(
                    "Місцевий час: {}",
                    local_time.format("%Y-%m-%d %H:%M:%S%.3f %:z")
                );

                println!("Статус: Успішно\n");

                successful += 1;

                let message = format!(
                    "SUCCESS | server={} | request={} | utc={}",
                    config.server,
                    number,
                    utc_time.format("%Y-%m-%d %H:%M:%S%.3f")
                );

                write_log_or_exit(&message);
            }

            Err(error) => {
                eprintln!("Помилка NTP: {}\n", error);

                failed += 1;

                let message = format!(
                    "ERROR | server={} | request={} | error={}",
                    config.server,
                    number,
                    error
                );

                write_log_or_exit(&message);
            }
        }

        if number < config.count {
            thread::sleep(
                Duration::from_secs(config.interval)
            );
        }
    }

    println!("====================================");
    println!("Успішних запитів: {}", successful);
    println!("Невдалих запитів: {}", failed);
    println!("Журнал: {}", logger::LOG_FILE);
    println!("Моніторинг завершено.");

    write_log_or_exit(&format!(
        "FINISH | successful={} | failed={}",
        successful, failed
    ));

    if failed > 0 {
        std::process::exit(1);
    }
}
