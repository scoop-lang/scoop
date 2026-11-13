package main

import (
	"syscall"
	"time"
	"unsafe"
)

// Go's Windows monotonic time uses interrupt ticks, which can quantize fast echoes to zero.
// Build this file with client.go to use QPC for both throughput and latency intervals.
func init() {
	kernel := syscall.NewLazyDLL("kernel32.dll")
	counter := kernel.NewProc("QueryPerformanceCounter")
	frequency := kernel.NewProc("QueryPerformanceFrequency")
	var ticksPerSecond int64
	if ok, _, err := frequency.Call(uintptr(unsafe.Pointer(&ticksPerSecond))); ok == 0 || ticksPerSecond <= 0 {
		panic(err)
	}
	read := func() int64 {
		var ticks int64
		if ok, _, err := counter.Call(uintptr(unsafe.Pointer(&ticks))); ok == 0 {
			panic(err)
		}
		return ticks
	}
	baseTicks, baseTime := read(), time.Now()
	clockNow = func() time.Time {
		delta := read() - baseTicks
		elapsed := time.Duration(delta/ticksPerSecond)*time.Second +
			time.Duration(delta%ticksPerSecond)*time.Second/time.Duration(ticksPerSecond)
		return baseTime.Add(elapsed)
	}
	clockName = "QueryPerformanceCounter"
	clockFrequency = ticksPerSecond
}
