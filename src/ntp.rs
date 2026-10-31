
use chrono::{DateTime, Utc};
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NTP_UNIX_OFFSET: i64 = 2_208_988_800;
const NTP_ERA_SECONDS: i64 = 1_i64 << 32;
const NANOS_PER_SECOND: i128 = 1_000_000_000;

// Результат одного NTP-запиту.
pub struct NtpResult {
    pub server_time: DateTime<Utc>,
    pub estimated_time: DateTime<Utc>,
    pub delay_ms: f64,
    pub offset_ms: f64,
}

// Перетворення системного часу в наносекунди Unix.
fn system_time_nanos(time: SystemTime) -> Result<i128, String> {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Некоректний системний час: {}", e))?;

    Ok(
        duration.as_secs() as i128 * NANOS_PER_SECOND
            + duration.subsec_nanos() as i128,
    )
}

// Перетворення наносекунд Unix у дату UTC.
fn nanos_to_datetime(nanos: i128) -> Result<DateTime<Utc>, String> {
    let seconds = i64::try_from(
        nanos.div_euclid(NANOS_PER_SECOND),
    )
    .map_err(|_| "Час виходить за допустимий діапазон")?;

    let fraction =
        nanos.rem_euclid(NANOS_PER_SECOND) as u32;

    DateTime::<Utc>::from_timestamp(seconds, fraction)
        .ok_or_else(|| "Не вдалося перетворити дату".to_string())
}

// Декодування 64-бітної часової позначки NTP.
fn decode_ntp_timestamp(
    bytes: &[u8],
    reference_unix_seconds: i64,
) -> Result<DateTime<Utc>, String> {
    if bytes.len() != 8 {
        return Err(
            "Некоректна довжина часової позначки".into()
        );
    }

    let seconds = u32::from_be_bytes(
        bytes[0..4]
            .try_into()
            .map_err(|_| "Помилка читання секунд NTP")?,
    ) as i64;

    let fraction = u32::from_be_bytes(
        bytes[4..8]
            .try_into()
            .map_err(|_| "Помилка читання дробової частини NTP")?,
    ) as u64;

    // Нульова позначка означає відсутність часу.
    if seconds == 0 && fraction == 0 {
        return Err("Порожня часова позначка NTP".into());
    }

    // Визначаємо еру NTP відносно часу клієнта.
    // Це дозволяє обробляти дати після 2036 року.
    let reference_ntp = reference_unix_seconds
        .checked_add(NTP_UNIX_OFFSET)
        .ok_or("Переповнення часу NTP")?;

    let era = (
        reference_ntp - seconds + NTP_ERA_SECONDS / 2
    )
    .div_euclid(NTP_ERA_SECONDS);

    let unix_seconds =
        era * NTP_ERA_SECONDS + seconds - NTP_UNIX_OFFSET;

    let nanos =
        ((fraction * 1_000_000_000) >> 32) as u32;

    DateTime::<Utc>::from_timestamp(unix_seconds, nanos)
        .ok_or_else(|| "Некоректна дата NTP".to_string())
}

// Обчислення мережевої затримки і зміщення часу.
//
// delay = (T4 - T1) - (T3 - T2)
// offset = ((T2 - T1) + (T3 - T4)) / 2
fn calculate_metrics(
    t1: i128,
    t2: i128,
    t3: i128,
    t4: i128,
) -> (i128, i128) {
    let delay = (t4 - t1) - (t3 - t2);

    let offset =
        ((t2 - t1) + (t3 - t4)) / 2;

    (delay, offset)
}

// Основна функція NTP-клієнта.
pub fn get_ntp_time(server: &str) -> Result<NtpResult, String> {
    // 1. Визначення IPv4-адреси NTP-сервера.
    let address = (server, 123)
        .to_socket_addrs()
        .map_err(|e| format!("Помилка DNS: {}", e))?
        .find(|addr| addr.is_ipv4())
        .ok_or("Не знайдено IPv4-адресу NTP-сервера")?;

    // 2. Створення UDP-сокета.
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| format!("Помилка створення сокета: {}", e))?;

    // Обмеження часу очікування відповіді.
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Помилка встановлення таймауту: {}", e))?;

    // Зв'язуємо сокет з адресою сервера.
    socket
        .connect(address)
        .map_err(|e| format!("Помилка підключення: {}", e))?;

    // 3. Формування NTPv4-пакета.
    let mut request = [0u8; 48];

    // LI = 0, Version = 4, Mode = 3 (Client).
    request[0] = 0x23;

    // T1 — час перед надсиланням запиту.
    let t1 = SystemTime::now();

    let duration = t1
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Помилка системного часу: {}", e))?;

    let ntp_seconds =
        (duration.as_secs() + NTP_UNIX_OFFSET as u64) as u32;

    let ntp_fraction =
        ((duration.subsec_nanos() as u64) << 32)
            / 1_000_000_000;

    // Записуємо T1 у Transmit Timestamp.
    request[40..44]
        .copy_from_slice(&ntp_seconds.to_be_bytes());

    request[44..48]
        .copy_from_slice(&(ntp_fraction as u32).to_be_bytes());

    // 4. Надсилання NTP-запиту.
    socket
        .send(&request)
        .map_err(|e| format!("Помилка надсилання NTP-запиту: {}", e))?;

    // 5. Отримання відповіді сервера.
    let mut response = [0u8; 512];

    let received = socket
        .recv(&mut response)
        .map_err(|e| {
            match e.kind() {
                std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::WouldBlock => {
                    "NTP-сервер не відповів протягом 5 секунд"
                        .to_string()
                }
                _ => format!(
                    "Помилка отримання NTP-відповіді: {}",
                    e
                ),
            }
        })?;

    // T4 — час отримання відповіді.
    let t4 = SystemTime::now();

    // 6. Перевірка NTP-пакета.
    // Використовуємо окремий модуль ntp_packet.rs.
    let sent_timestamp: [u8; 8] = request[40..48]
        .try_into()
        .map_err(|_| "Некоректна часова позначка запиту")?;

    crate::ntp_packet::validate_response(
        &response[..received],
        &sent_timestamp,
    )?;

    // 7. Отримання часових позначок T2 і T3.
    let t4_nanos = system_time_nanos(t4)?;

    let reference_seconds = i64::try_from(
        t4_nanos.div_euclid(NANOS_PER_SECOND),
    )
    .map_err(|_| "Некоректний системний час")?;

    // T2 — час отримання запиту сервером.
    let receive_time = decode_ntp_timestamp(
        &response[32..40],
        reference_seconds,
    )?;

    // T3 — час надсилання відповіді сервером.
    let transmit_time = decode_ntp_timestamp(
        &response[40..48],
        reference_seconds,
    )?;

    // 8. Перетворення часових позначок у наносекунди.
    let t1_nanos = system_time_nanos(t1)?;

    let t2_nanos =
        receive_time.timestamp() as i128 * NANOS_PER_SECOND
            + receive_time.timestamp_subsec_nanos() as i128;

    let t3_nanos =
        transmit_time.timestamp() as i128 * NANOS_PER_SECOND
            + transmit_time.timestamp_subsec_nanos() as i128;

    // Час відправлення відповіді не може передувати
    // часу її отримання сервером.
    if t3_nanos < t2_nanos {
        return Err(
            "Некоректний порядок часових позначок NTP".into()
        );
    }

    // 9. Обчислення мережевої затримки та зміщення.
    let (delay_ns, offset_ns) = calculate_metrics(
        t1_nanos,
        t2_nanos,
        t3_nanos,
        t4_nanos,
    );

    if delay_ns < 0 {
        return Err(
            "Некоректна від'ємна мережева затримка".into()
        );
    }

    // 10. Обчислення оцінки поточного часу UTC
    // на момент отримання відповіді T4.
    let estimated_time =
        nanos_to_datetime(t4_nanos + offset_ns)?;

    Ok(NtpResult {
        server_time: transmit_time,
        estimated_time,
        delay_ms: delay_ns as f64 / 1_000_000.0,
        offset_ms: offset_ns as f64 / 1_000_000.0,
    })
}

// Автоматичні тести.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_zero_offset() {
        let (delay, offset) = calculate_metrics(
            1_000_000_000,
            1_050_000_000,
            1_070_000_000,
            1_120_000_000,
        );

        assert_eq!(delay, 100_000_000);
        assert_eq!(offset, 0);
    }

    #[test]
    fn calculates_positive_offset() {
        let (delay, offset) = calculate_metrics(
            0,
            60_000_000,
            80_000_000,
            100_000_000,
        );

        assert_eq!(delay, 80_000_000);
        assert_eq!(offset, 20_000_000);
    }

    #[test]
    fn rejects_empty_timestamp() {
        let result = decode_ntp_timestamp(
            &[0u8; 8],
            1_800_000_000,
        );

        assert!(result.is_err());
    }

    #[test]
    fn handles_ntp_era_after_2036() {
        let unix_seconds = 2_200_000_000_i64;

        let ntp_seconds =
            (unix_seconds + NTP_UNIX_OFFSET) as u32;

        let mut bytes = [0u8; 8];

        bytes[0..4]
            .copy_from_slice(&ntp_seconds.to_be_bytes());

        let decoded = decode_ntp_timestamp(
            &bytes,
            unix_seconds,
        )
        .unwrap();

        assert_eq!(decoded.timestamp(), unix_seconds);
    }
}
