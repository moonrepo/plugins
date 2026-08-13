module example.com/org/replacer

require (
    example.com/org/renamed v1.0.0
    example.com/org/mod v1.0.0
    example.com/org/outside v1.0.0
)

replace example.com/org/renamed => ../arbitrary

replace example.com/org/mod => example.com/external/mod v1.0.0

replace example.com/org/outside => ../../outside
