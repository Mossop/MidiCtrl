# MidiCtrl

An experimental tool to wire up a MIDI controller to Lightroom.

Yes, many of these already exist, none really did exactly what I wanted though and I quickly found
them frustrating to use. Mostly it was about configurability and ability to feedback to the control's display.

Plus I've been looking for a good excuse to get back into Rust.

This is split into two parts, a Lightroom plugin for getting the current Lightroom state and making changes and a Rust binary for talking to the plugin and any MIDI controllers.

# Configuration

Configuration is done entirely via JSON files. There is no UI here. Maybe one would be nice but the capabilities of this plugin are complicated to translate to simple UI.

There are two types of configuration, devices and profiles. Device configuration describes the MIDI devices, profile configuration describes how to map between devices and Lightroom. You can switch between profiles while Lightroom is running, if you change your attached devices you must restart.

Everything is based around the current state of Lightroom. The state is a set of parameters, each having a name and a value which may be a number, string or boolean. Controls on the MIDI devices can modify these parameters and as the parameters change so the displays on the MIDI controllers can change.

## Device configuration

The `devices` directory in the settings directory contains one JSON file for each MIDI device.
```
{
  "name": "X-TOUCH MINI",
  "controls": [
    {
      "name": "Encoder 1",
      "type": "cc",
      "layers": {
        "A": {
          "channel": 11,
          "control": 1,
          "min": 0,
          "max": 127
        }
      }
    },
    {
      "name": "Button 1",
      "type": "key",
      "display": true,
      "layers": {
        "A": {
          "channel": 11,
          "note": 8,
          "off": 0,
          "on": 127
        }
      }
    }
  ]
}
```

A device has a name (matches the exposed MIDI name) and a set of controls. Each control can be a continuous control (cc, like knobs or faders) or a key (like a button). Some devices have selectable layers so the same control may be configured differently in different layers. Layers are basically ignored for now though.

## Profile configuration

The `profiles` directory in the settings directory contains one JSON file for each profile.
```
{
  "if": { "parameter": "module", "value": "develop" },
  "controls": [
    {
      "device": "x-touch-mini",
      "layer": "A",
      "control": "Encoder 1",
      "action": "Exposure"
    },
    {
      "device": "x-touch-mini",
      "layer": "A",
      "control": "Button 1",
      "source": { "condition": { "parameter": "module", "value": "develop" } },
      "action": [
        { "if": { "parameter": "module", "value": "develop" }, "then": { "parameter": "module", "value": "library" } },
        { "parameter": "module", "value": "develop" }
      ]
    },
  ]
}
```
The `if` property controls whether the profile is available, it is a condition as described below. The controls map to the device name (name of the JSON file) and the specific control and layer.

Controls must have an `action` (what they do when used) and may have a `source` (controls when their display is updated). If a control's `action` is simply setting the value of a parameter than the value of that parameter is used as the `source` by default.

For continuous controls the source must resolve to a number between 0 and 1. For buttons it must resolve to a boolean.

Both sources and actions may be conditional, an object with an `if` condition and a `then` result. They may also be arrays of such, the first one that matches the condition is the result. In the `Button 1` example above pressing the button will change the module to `library` if it is currently `develop` otherwise it will change the module to `develop`. The final set of actions may also be an array allowing a single button to trigger multiple effects.

## Sources

There are a few different ways to provide the display for a control. Remember that for continuous controls the source must end as a number from 0 to 1, for a button it must be a boolean.

The value of the parameter `Exposure` is used:
```
"source": "Exposure"
```

The boolean parameter `isRejected` is inverted:
```
"source": { "parameter": "isRejected", "inverted": true },
```

The value 0.5 is used (continuous controls).
```
"source": 0.5
```

The value is true (buttons).
```
"source": true
```


If the condition (see below) matches then the value is true. The `invert` property can be used to invert the result but may be left out if false.
```
"source": { "condition": { /* condition */ }, "invert": false }
```

## Actions

Continuous controls can do just one thing, set the value of a numeric parameter:
```
"action": "Exposure"
```

Buttons have a few options...

Sets a boolean paramater to true when pressed:
```
"action": "isRejected"
```

Toggles a boolean parameter when pressed:
```
"action": { "toggle": "isRejected" }
```

Sets a parameter to something specific:
```
"action": { "parameter": "Exposure", "value": 0.5 }
```

## Conditions

Conditions can be used to disable profiles and configure actions and sources. There is one basic condition:
```
{ "parameter": "Exposure", "comparison": ">", "value": 0.5 }
```
This tests that the parameter `Exposure` is larger than 0.5. The comparison may be `==`, `!=`, `<`, `>`, `<=` or `>=`. It may be left out entirely for the `==` case. If the given `value` has a different type than the parameter than the comparison fails (except for `!=`).

You an also combine conditions:
```
{
  "any": [
    { "parameter": "Exposure", "comparison": ">", "value": 0.5 },
    { "parameter": "Exposure", "comparison": "<", "value": -0.5 },
  ]
}
```
```
{
  "all": [
    { "parameter": "Exposure", "comparison": ">", "value": -0.5 },
    { "parameter": "Exposure", "comparison": "<", "value": 0.5 },
  ],
  "invert": true
}
```
Both accept the `invert` property but it may be left off if it is `false`.
