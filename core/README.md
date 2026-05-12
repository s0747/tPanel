# Wszystkie czujniki, ostatnia godzina
GET /api/range?from=1744000000

# Tylko sensor-01
GET /api/range?from=1744000000&sensor_id=sensor-01

# Lista wszystkich czujników w bazie
GET /api/sensors
→ ["sensor-01", "room-temp", "outdoor"]

GET    /api/panels/{panel_uuid}/sensors              → lista czujników panelu
POST   /api/panels/{panel_uuid}/sensors              → { "sensor_uuid": "abc", "name": "Salon" }
DELETE /api/panels/{panel_uuid}/sensors/{sensor_uuid} → usuń czujnik z panelu