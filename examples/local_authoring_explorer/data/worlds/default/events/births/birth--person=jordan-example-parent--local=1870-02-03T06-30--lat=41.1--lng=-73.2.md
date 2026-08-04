+++
schema_version = 1
id = "event:birth-jordan-example-parent"
kind = "event"
participants = ["self"]

[location]
label = "Example Parent Birthplace"

[[assertions]]
target = "#datetime"
confidence = "medium"
sources = [
  { label = "Example parent birth note", kind = "family-note", file = "media/sources/jordan-example-parent-birth-note.jpg" },
]
note = "Filename supplies the local date/time and coordinates; this fictional note supports the combined datetime."
+++

# Jordan Example Parent was born

Minimal parent birth event. The filename supplies type, participant, local datetime, and coordinates; the frontmatter keeps the label and evidence easy to edit.
