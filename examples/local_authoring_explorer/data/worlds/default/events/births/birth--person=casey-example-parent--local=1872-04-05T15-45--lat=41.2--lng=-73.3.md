+++
schema_version = 1
id = "event:birth-casey-example-parent"
kind = "event"
participants = ["self"]

[location]
label = "Another Example Parent Birthplace"

[[assertions]]
target = "#datetime"
confidence = "medium"
sources = [
  { label = "Example parent birth certificate photo", kind = "birth-certificate", file = "media/sources/casey-example-parent-birth-certificate.jpg" },
]
note = "Filename supplies the local date/time and coordinates; this fictional certificate supports the combined datetime."
+++

# Casey Example Parent was born

Another compact parent birth event using filename hints for the machine-friendly birth details.
