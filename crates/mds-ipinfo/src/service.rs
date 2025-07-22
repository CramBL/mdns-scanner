use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceInstance {
    pub(crate) name: String,
    // Only applicable if it advertises an mDNS hostname by itself that doesn't match the hostname of the host at the IP its at
    pub(crate) hostname: Option<String>,
    pub(crate) _type: String,
    pub(crate) port: u16,
    pub(crate) txt: Option<Vec<String>>,
}

impl ServiceInstance {
    pub fn new(
        name: String,
        _type: String,
        hostname: Option<String>,
        port: u16,
        txt: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            hostname,
            _type,
            port,
            txt,
        }
    }

    pub fn remove_hostname_if_contained_in(&mut self, names: &[String]) {
        let _ = self.hostname.take_if(|h| names.contains(h));
    }
}

impl fmt::Display for ServiceInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.name;
        let host_opt = self
            .hostname
            .as_deref()
            .map(|h| format!(" @ {h}"))
            .unwrap_or_default();
        let port = self.port;

        write!(f, "{name}{host_opt}:{port}")?;

        if let Some(txt_records) = &self.txt {
            const MAX_LINE_LENGTH: usize = 65;
            let mut current_line = String::new();

            for (i, record) in txt_records.iter().enumerate() {
                let prefix = if i == 0 { "\n" } else { ", " };
                let new_content = format!("{prefix}{record}");

                if !current_line.is_empty()
                    && current_line.len() + new_content.len() > MAX_LINE_LENGTH
                {
                    writeln!(f, "{current_line}")?;
                    current_line = record.to_string();
                } else {
                    current_line.push_str(&new_content);
                }
            }

            write!(f, "{current_line}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_str_eq;

    #[test]
    fn displays_name_and_port() {
        let service = ServiceInstance::new(
            "test".to_string(),
            "_http._tcp".to_string(),
            None,
            8080,
            None,
        );
        assert_str_eq!(service.to_string(), "test:8080");
    }

    #[test]
    fn displays_hostname_when_present() {
        let service = ServiceInstance::new(
            "test".to_string(),
            "_http._tcp".to_string(),
            Some("host.local".to_string()),
            8080,
            None,
        );
        assert_str_eq!(service.to_string(), "test @ host.local:8080");
    }

    #[test]
    fn displays_single_txt_record() {
        let service = ServiceInstance::new(
            "test".to_string(),
            "_http._tcp".to_string(),
            None,
            8080,
            Some(vec!["key=value".to_string()]),
        );
        assert_str_eq!(service.to_string(), "test:8080\nkey=value");
    }

    #[test]
    fn displays_multiple_txt_records_on_same_line() {
        let service = ServiceInstance::new(
            "test".to_string(),
            "_http._tcp".to_string(),
            None,
            8080,
            Some(vec!["a=1".to_string(), "b=2".to_string()]),
        );
        assert_str_eq!(service.to_string(), "test:8080\na=1, b=2");
    }

    #[test]
    fn wraps_long_txt_records() {
        let long_record = "x".repeat(55);
        let service = ServiceInstance::new(
            "test".to_string(),
            "_http._tcp".to_string(),
            None,
            8080,
            Some(vec![
                "short".to_string(),
                long_record.clone(),
                long_record.clone(),
            ]),
        );
        let expected = format!("test:8080\nshort, {long_record}\n{long_record}");
        assert_str_eq!(service.to_string(), expected);
    }

    #[test]
    fn handles_txt_record_exactly_at_line_limit() {
        let record = "x".repeat(65);
        let service = ServiceInstance::new(
            "test".to_string(),
            "_http._tcp".to_string(),
            None,
            8080,
            Some(vec![record.clone()]),
        );
        assert_str_eq!(service.to_string(), format!("test:8080\n{record}"));
    }

    #[test]
    fn wraps_when_adding_comma_exceeds_limit() {
        let record1 = "x".repeat(63);
        let record2 = "y".to_string();
        let service = ServiceInstance::new(
            "test".to_string(),
            "_http._tcp".to_string(),
            None,
            8080,
            Some(vec![record1.clone(), record2.clone()]),
        );
        let expected = format!("test:8080\n{record1}\n{record2}");
        assert_str_eq!(service.to_string(), expected);
    }

    #[test]
    fn combines_all_elements() {
        let service = ServiceInstance::new(
            "web".to_string(),
            "_http._tcp".to_string(),
            Some("server.local".to_string()),
            443,
            Some(vec!["ssl=true".to_string(), "version=2.0".to_string()]),
        );
        assert_str_eq!(
            service.to_string(),
            "web @ server.local:443\nssl=true, version=2.0"
        );
    }
}
