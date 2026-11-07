# proto plugin

A built-in plugin for installing proto itself. proto uses it when running `proto upgrade`, and when
a version of proto is pinned with the `proto` key in `.prototools`. It can't be overridden, as the
`proto` identifier is reserved.
