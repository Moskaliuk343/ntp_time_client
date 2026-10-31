
const MIN_PACKET_SIZE: usize = 48;

/// Перевіряє NTP-відповідь перед обробкою часу.
pub fn validate_response(
    response: &[u8],
    sent_timestamp: &[u8; 8],
) -> Result<(), String> {
    // Перевірка мінімальної довжини пакета.
    if response.len() < MIN_PACKET_SIZE {
        return Err(format!(
            "Занадто короткий NTP-пакет: {} байтів",
            response.len()
        ));
    }

    // Перший байт містить LI, Version та Mode.
    let leap = response[0] >> 6;
    let version = (response[0] >> 3) & 0x07;
    let mode = response[0] & 0x07;
    let stratum = response[1];

    // Підтримуємо відповіді NTPv3 і NTPv4.
    if !(3..=4).contains(&version) {
        return Err(format!(
            "Непідтримувана версія NTP: {}",
            version
        ));
    }

    // Серверна відповідь повинна мати Mode = 4.
    if mode != 4 {
        return Err(format!(
            "Некоректний режим NTP: {}",
            mode
        ));
    }

    // Stratum 0: сервер повідомляє про особливий стан.
    if stratum == 0 {
        let code = String::from_utf8_lossy(&response[12..16]);

        return Err(format!(
            "Сервер повернув Kiss-o'-Death: {}",
            code
        ));
    }

    // LI = 3 означає несинхронізований сервер.
    if leap == 3 {
        return Err(
            "NTP-сервер повідомляє про відсутність синхронізації"
                .into()
        );
    }

    if stratum > 15 {
        return Err(format!(
            "Некоректний рівень Stratum: {}",
            stratum
        ));
    }

    // Originate Timestamp має збігатися з T1 запиту.
    if response[24..32] != sent_timestamp[..] {
        return Err(
            "Отримана відповідь не відповідає NTP-запиту"
                .into()
        );
    }

    // Перевіряємо T2 і T3.
    if response[32..40].iter().all(|byte| *byte == 0) {
        return Err(
            "Відповідь не містить часу отримання запиту T2"
                .into()
        );
    }

    if response[40..48].iter().all(|byte| *byte == 0) {
        return Err(
            "Відповідь не містить часу відправлення T3"
                .into()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Створюємо тестову правильну відповідь.
    fn valid_packet() -> ([u8; 48], [u8; 8]) {
        let mut response = [0u8; 48];

        let timestamp = [1, 2, 3, 4, 5, 6, 7, 8];

        response[0] = 0x24; // NTPv4, server mode
        response[1] = 2;    // Stratum 2

        response[24..32].copy_from_slice(&timestamp);

        response[39] = 1; // Ненульовий T2
        response[47] = 2; // Ненульовий T3

        (response, timestamp)
    }

    #[test]
    fn accepts_valid_packet() {
        let (packet, timestamp) = valid_packet();

        assert!(validate_response(&packet, &timestamp).is_ok());
    }

    #[test]
    fn rejects_short_packet() {
        let (_, timestamp) = valid_packet();

        let packet = [0u8; 20];

        assert!(validate_response(&packet, &timestamp).is_err());
    }

    #[test]
    fn rejects_wrong_mode() {
        let (mut packet, timestamp) = valid_packet();

        packet[0] = 0x23; // Client mode замість Server

        assert!(validate_response(&packet, &timestamp).is_err());
    }

    #[test]
    fn rejects_unsupported_version() {
        let (mut packet, timestamp) = valid_packet();

        packet[0] = 0x14; // NTPv2, Server mode

        assert!(validate_response(&packet, &timestamp).is_err());
    }

    #[test]
    fn rejects_unsynchronized_server() {
        let (mut packet, timestamp) = valid_packet();

        packet[0] = 0xE4; // LI = 3

        assert!(validate_response(&packet, &timestamp).is_err());
    }

    #[test]
    fn rejects_kiss_of_death() {
        let (mut packet, timestamp) = valid_packet();

        packet[1] = 0;
        packet[12..16].copy_from_slice(b"RATE");

        let error =
            validate_response(&packet, &timestamp).unwrap_err();

        assert!(error.contains("RATE"));
    }

    #[test]
    fn rejects_invalid_stratum() {
        let (mut packet, timestamp) = valid_packet();

        packet[1] = 16;

        assert!(validate_response(&packet, &timestamp).is_err());
    }

    #[test]
    fn rejects_wrong_originate_timestamp() {
        let (mut packet, timestamp) = valid_packet();

        packet[24] = 99;

        assert!(validate_response(&packet, &timestamp).is_err());
    }

    #[test]
    fn rejects_missing_timestamps() {
        let (mut packet, timestamp) = valid_packet();

        packet[32..40].fill(0);

        assert!(validate_response(&packet, &timestamp).is_err());

        let (mut packet, timestamp) = valid_packet();

        packet[40..48].fill(0);

        assert!(validate_response(&packet, &timestamp).is_err());
    }
}
