// Deprecated: use v2 instead
module github.com/org/repo

go 1.24.0

toolchain go1.21.0

require (
	example.com/new/thing/v2 v2.3.4
	example.com/old/thing v1.2.3
)

// Other comment

exclude example.com/old/thing v1.2.3

replace example.com/bad/thing v1.4.5 => example.com/good/thing v1.4.5

retract [v1.9.0, v1.9.5]
