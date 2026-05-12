use serde::{Deserialize, Serialize};

/// Punkt pomiarowy z tematu MQTT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub ts:       f64,
    pub temp:     f64,
    pub humidity: f64,
}

/// Punkt z sensor_id (do zapisu w bazie)
#[derive(Debug, Clone)]
pub struct ReadingRecord {
    pub sensor_id: String,
    pub ts:        f64,
    pub temp:      f64,
    pub humidity:  f64,
}

/// Odpowiedź HTTP GET /api/range
#[derive(Debug, Serialize)]
pub struct RangeResponse {
    pub from:      f64,
    pub to:        f64,
    pub sensor_id: Option<String>,
    pub count:     usize,
    pub points:    Vec<Reading>,
}

/// Wyciąga sensor_id z nazwy topicu.
/// Format: sensors/{sensor_id}/data → "sensor_id"
/// Zwraca "default" gdy format nie pasuje.
pub fn sensor_id_from_topic(topic: &str) -> String {
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() >= 2 {
        parts[1].to_owned()
    } else {
        "default".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_sensor_id() {
        assert_eq!(sensor_id_from_topic("sensors/s1/data"), "s1");
        assert_eq!(sensor_id_from_topic("sensors/room/data"), "room");
        assert_eq!(sensor_id_from_topic("sensors/only"), "only");
        assert_eq!(sensor_id_from_topic("single"), "default");
    }
}
