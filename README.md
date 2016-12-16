Goals
===
* Fast
* Precise parsing error messages with dumps etc.
* Dead letter box with message reprocessing tools
* Programable
* Input/filter/output library including syslog, Kafka, filed renaming, value mapping etc.

Design
===
* input - crates a stream of concrete structures that implements Event trait
* output - map the stream of abstract Events and apply concrete Serializer to it to produce final objects into destination
* serializer - builds Serializers to be attached to outputs
* mapper - functions work with streams of events - eg. by grouping them together or routing to different outputs
* ports - Port traits are defined for each output type and need to be implemented on concreate Event type to be accepted for that output

Input
---
Takes data from somewhere (e.g. TCP Stream) and produces Stream of concrete envet types depending on the input.

Serializer
---
Role of Serializer is to provide byte stream representaion of Event for the outputs.

Custom Serializers are build using builder like API. Ther resulting final object implements Serializer.
Serializer can be used to process many messages to Write type.

Output
---
They map the events applying Serializer that write them to final destination.
They return event stream that they take so they can be chanied together.
Event loop needs to pull that stream to get items through outputs.

Port
---
Ports are traits that are defined per each output. They role is to provide all information needed for the output from events, e.g. ID, channel, topic, etc..

Events can be wrapped in specialized types that implement given Port for functionality like topic load balancing etc..

Event Types
---
Event types are trais that provide common base infomration about event liek timestamp, version, or extre fields.
Outputs will require prticular event type trait implemented on messges they receive.

Logslash Events
---
Serialized with SerDe to all it's supported formats.
Every Input type implements Event trait and each Port is implemented for it.
