use mds_ipinfo::IpInfo;
use unicode_width::UnicodeWidthStr;

pub(crate) struct ColumnConstraints {
    pub(crate) max_ip_len: u16,
    pub(crate) max_hostname_len: u16,
    pub(crate) max_packets_count_len: u16,
    pub(crate) max_services_len: u16,
}

impl Default for ColumnConstraints {
    fn default() -> Self {
        Self {
            max_ip_len: 10,
            max_hostname_len: 10,
            max_packets_count_len: 10,
            max_services_len: 10,
        }
    }
}

impl ColumnConstraints {
    pub fn new(items: &[&IpInfo]) -> Self {
        let mut max_ip_len: u16 = 0;
        let mut max_hostname_len: u16 = 0;
        let mut max_packets_count_len: u16 = 0;
        let mut max_services_len: u16 = 0;

        for it in items {
            let tmp_ip_len = it.ip.max_unicode_width();
            if max_ip_len < tmp_ip_len {
                max_ip_len = tmp_ip_len;
            }

            let tmp_hostname_len = it.max_name_unicode_width();
            if max_hostname_len < tmp_hostname_len {
                max_hostname_len = tmp_hostname_len;
            }

            let tmp_packets_count_len = it.seen_count().to_string().width() as u16;
            if max_packets_count_len < tmp_packets_count_len {
                max_packets_count_len = tmp_packets_count_len;
            }

            let tmp_services_len = it.max_service_instance_unicode_width();
            if max_services_len < tmp_services_len {
                max_services_len = tmp_services_len;
            }
        }

        Self {
            max_ip_len,
            max_hostname_len,
            max_packets_count_len,
            max_services_len,
        }
    }
}
