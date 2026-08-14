package consumer

import (
	modv1 "example.com/org/mod"
	modv2 "example.com/org/mod/v2"
)

func Consume() {
	modv1.Old()
	modv2.New()
}
