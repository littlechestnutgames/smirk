use std::time::SystemTime;

pub struct Record<T> {
    pub value: T,
    pub ttl: Option<u64>,
    pub ttl_start: SystemTime,
    pub type_name: String,
    pub desired_type_name: String
}

pub trait RecordLike<T> {
    fn is_expired(&self) -> bool;
    fn get_ttl(&self) -> Option<u64>;
}

impl<T> RecordLike<T> for Record<T> {
    fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl  {
            return SystemTime::now()
                .duration_since(self.ttl_start)
                .unwrap_or_default()
                .as_secs() >= ttl;
        }

        false
    }
    fn get_ttl(&self) -> Option<u64> {

        if let Some(ttl) = self.ttl  {
            let duration = SystemTime::now()
                .duration_since(self.ttl_start)
                .unwrap_or_default()
                .as_secs();
            if duration >= ttl {
                return Some(0);
            }
            return Some(ttl - duration);
        }
        None
    }
}
