+++
schema_version = 1
id = "event:birth-riley-example-inline"
kind = "event"
type = "birth"
time = "1905-05-06 04:32"
date_precision = "minute"
participants = ["riley-example"]
sources = [
  { label = "Example birth document photo", kind = "birth-record", file = "media/sources/riley-example-birth-document.jpg" },
]

[location]
label = "Example City Birth Center"
source_text = "Example City Birth Center, Example Region"
latitude = 40.7128
longitude = -74.0060

[[assertions]]
target = "#date"
sources = [
  { label = "Example birth document photo", kind = "birth-record", file = "media/sources/riley-example-birth-document.jpg" },
]
confidence = "high"
note = "The fictional birth document gives a complete date and local time."

[[assertions]]
target = "#time"
sources = [
  { label = "Example birth document photo", kind = "birth-record", file = "media/sources/riley-example-birth-document.jpg" },
]
confidence = "high"
note = "The fictional birth document gives the recorded birth time."

[[assertions]]
target = "#location"
sources = [
  { label = "Example birth document photo", kind = "birth-record", file = "media/sources/riley-example-birth-document.jpg" },
]
confidence = "high"
note = "The fictional birth document names the location; coordinates are hand-entered for exploration."
+++

# Riley Example was born

This fictional event demonstrates the most convenient local authoring style: the participant, time, source support, and full inline location are all kept in one event file.
