# Rusty Presquile
Write podcast chapter to mp3 id3V2 tags from Adobe Audition CSV file.

This repo it is none other than a Rust porting of [brainrepo/presquile](https://github.com/brainrepo/presquile).

# Prerequisites
See [zmwangx/metadata build dependencies ](https://github.com/zmwangx/metadata#building-from-source)

# Benchmark
![Density](./resources/bench/pdf_small.svg)
![Samples](./resources/bench/iteration_times_small.svg)

# Usage 

```
./presquile
Usage: presquile <AUDITION_CVS> <MP3_FILE> <COMMAND>

Commands:
  apply  Write chapter to mp3 id3V2 tags from Adobe Audition CSV file
  help   Print this message or the help of the given subcommand(s)

Arguments:
  <AUDITION_CVS>  Audition CVS Markers file
  <MP3_FILE>      Mp3 file

Options:
  -h, --help     Print help
  -V, --version  Print version
```