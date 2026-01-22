![Logo](https://github.com/Karspexet/ClicKS/blob/294e49c50a60a88f4bc0d656fe234b8173b2a486/images/icon-cropped.png)

# ClicKS

ClicKS is a metronome and timing control system developed by Axel Stenberg for use by Kårspexet.


## Features

- Headless Linux host powered by JACK audio server through an external audio device
- Lightweight live monitoring and control software for Windows/Linux/MacOS over network
- Dedicated show programmer and configuration editor
- Integration with OSC, JSON and SMPTE timecode
- Low latency transport and playback control; allows for real-time musical integration
## Installation
To install ClicKS Monitor and/or ClicKS Editor, see [Releases](https://github.com/Karspexet/ClicKS/releases)
## Run Locally
To run the ClicKS host locally, follow these steps:

Clone the project

```bash
  git clone https://github.com/Karspexet/ClicKS
```

Go to the host directory

```bash
  cd ClicKS/host
```

The host requires a show configuration file to start. Either create one using ClicKS Editor, or place a `show.json` with the following contents in a directory named `clicks.show` anywhere on your device:

```
{"metadata":{"name":"","date":"","credits":[]},"cues":[{"metadata":{"name":"Unnamed Cue","human_ident":"0"},"beats":[{"count":1,"bar_number":1,"length":500000,"events":[]},{"count":2,"bar_number":1,"length":500000,"events":[]},{"count":3,"bar_number":1,"length":500000,"events":[]},{"count":4,"bar_number":1,"length":500000,"events":[]},{"count":1,"bar_number":2,"length":500000,"events":[]},{"count":2,"bar_number":2,"length":500000,"events":[]},{"count":3,"bar_number":2,"length":500000,"events":[]},{"count":4,"bar_number":2,"length":500000,"events":[]},{"count":1,"bar_number":3,"length":500000,"events":[]},{"count":2,"bar_number":3,"length":500000,"events":[]},{"count":3,"bar_number":3,"length":500000,"events":[]},{"count":4,"bar_number":3,"length":500000,"events":[]},{"count":1,"bar_number":4,"length":500000,"events":[]},{"count":2,"bar_number":4,"length":500000,"events":[]},{"count":3,"bar_number":4,"length":500000,"events":[]},{"count":4,"bar_number":4,"length":500000,"events":[]}]}]}
```

Build and run the host

```bash
  cargo run
```

Use ClicKS Monitor to connect to the host process and verify it's running correctly.
