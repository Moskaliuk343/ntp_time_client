
use chrono::{DateTime, Utc};
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Різниця між початком епох NTP (1900)
// та Unix (1970) у секундах.
const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

pub fn get_ntp_time(server: &str) -> Result<DateTime<Utc>, String> {
    // 1. Знаходимо IPv4-адресу NTP-сервера.
    let address = (server, 123)
        .to_socket_addrs()
        .map_err(|e| format!("Помилка DNS: {}", e))?
        .find(|addr| addr.is_ipv4())
        .ok_or("Не знайдено IPv4-адресу сервера")?;

    // 2. Створюємо UDP-сокет.
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| format!("Помилка створення сокета: {}", e))?;

    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Помилка таймауту: {}", e))?;

    socket
        .connect(address)
        .map_err(|e| format!("Помилка підключення: {}", e))?;

    // 3. Формуємо NTP-пакет із 48 байтів.
    let mut request = [0u8; 48];

    // LI = 0, Version = 4, Mode = 3 (Client).
    request[0] = 0x23;

    // Записуємо час відправлення запиту (T1).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Помилка системного часу: {}", e))?;

    let ntp_seconds = now.as_secs() + NTP_UNIX_OFFSET;

    let ntp_fraction =
        ((now.subsec_nanos() as u64) << 32) / 1_000_000_000;

    let seconds_bytes = (ntp_seconds as u32).to_be_bytes();
    let fraction_bytes = (ntp_fraction as u32).to_be_bytes();

    request[40..44].copy_from_slice(&seconds_bytes);
    request[44..48].copy_from_slice(&fraction_bytes);

    // 4. Надсилаємо запит.
    socket
        .send(&request)
        .map_err(|e| format!("Помилка надсилання: {}", e))?;

    // 5. Отримуємо відповідь.
    let mut response = [0u8; 512];

    let received = socket
        .recv(&mut response)
        .map_err(|e| format!("Помилка отримання відповіді: {}", e))?;

    if received < 48 {
        return Err("NTP-відповідь занадто коротка".into());
    }

    // 6. Перевіряємо коректність відповіді.
    let leap = response[0] >> 6;
    let version = (response[0] >> 3) & 0x07;
    let mode = response[0] & 0x07;
    let stratum = response[1];

    if mode != 4 || !(3..=4).contains(&version) {
        return Err("Некоректний режим або версія NTP".into());
    }

    if leap == 3 || !(1..=15).contains(&stratum) {
        return Err("NTP-сервер не надає синхронізований час".into());
    }

    // Сервер повинен повернути наш час T1
    // у полі Originate Timestamp.
    if response[24..32] != request[40..48] {
        return Err("NTP-відповідь не відповідає запиту".into());
    }

    // 7. Читаємо Transmit Timestamp (T3) сервера.
    let seconds = u32::from_be_bytes(
        response[40..44].try_into().unwrap()
    ) as u64;

    let fraction = u32::from_be_bytes(
        response[44..48].try_into().unwrap()
    ) as u64;

    if seconds < NTP_UNIX_OFFSET {
        return Err("Непідтримувана епоха NTP".into());
    }

    if seconds == 0 && fraction == 0 {
        return Err("Порожня позначка часу NTP".into());
    }

    // 8. Перетворюємо час NTP на Unix.
    let unix_seconds = (seconds - NTP_UNIX_OFFSET) as i64;

    let nanos = ((fraction * 1_000_000_000) >> 32) as u32;

    DateTime::<Utc>::from_timestamp(unix_seconds, nanos)
        .ok_or_else(|| "Не вдалося перетворити NTP-час".into())
}
