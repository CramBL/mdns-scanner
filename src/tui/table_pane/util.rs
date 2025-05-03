use crate::ip_info::IpInfo;
use unicode_width::UnicodeWidthStr;

pub(crate) fn constraint_len_calculator(items: &[&IpInfo]) -> (u16, u16, u16, u16) {
    let ip_len = items
        .iter()
        .map(|m| m.ip().to_string().width())
        .max()
        .unwrap_or(0);

    let hostname_len = items
        .iter()
        .map(|m| m.max_name_unicode_width())
        .max()
        .unwrap_or(0);

    let packets_count_len = items
        .iter()
        .map(|m| m.seen_count().to_string().width())
        .max()
        .unwrap_or(0);

    let services_len = items
        .iter()
        .map(|s| s.max_service_instance_unicode_width())
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (
        ip_len as u16,
        hostname_len,
        packets_count_len as u16,
        services_len,
    )
}
