package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net"
	"os"
	"runtime"
	"strconv"
	"sync"
	"sync/atomic"
)

func number(index int) int {
	value, err := strconv.Atoi(os.Args[index])
	if err != nil || value <= 0 {
		panic("arguments must be positive integers")
	}
	return value
}

func metrics() {
	var m runtime.MemStats
	runtime.ReadMemStats(&m)
	value, err := json.Marshal(map[string]any{
		"allocated_bytes": m.TotalAlloc,
		"collections":     m.NumGC,
		"pause_ns":        m.PauseTotalNs,
	})
	if err != nil {
		panic(err)
	}
	fmt.Println("METRICS", string(value))
}

func main() {
	if len(os.Args) != 5 {
		panic("usage: server WORKERS PORT CONNECTIONS BUFFER_BYTES")
	}
	runtime.GOMAXPROCS(number(1))
	port, connections, bufferSize := number(2), number(3), number(4)
	metrics()
	listener, err := net.ListenTCP("tcp4", &net.TCPAddr{IP: net.IPv4(127, 0, 0, 1), Port: port})
	if err != nil {
		panic(err)
	}
	fmt.Println("READY")
	var group sync.WaitGroup
	var failures atomic.Int64
	for i := 0; i < connections; i++ {
		peer, err := listener.AcceptTCP()
		if err != nil {
			panic(err)
		}
		if err := peer.SetNoDelay(true); err != nil {
			panic(err)
		}
		group.Add(1)
		go func() {
			defer group.Done()
			defer peer.Close()
			buffer := make([]byte, bufferSize)
			for {
				count, readErr := peer.Read(buffer)
				for written := 0; written < count; {
					n, err := peer.Write(buffer[written:count])
					if err != nil || n == 0 {
						failures.Add(1)
						return
					}
					written += n
				}
				if readErr != nil {
					if readErr != io.EOF {
						failures.Add(1)
					}
					return
				}
			}
		}()
	}
	if err := listener.Close(); err != nil {
		panic(err)
	}
	group.Wait()
	metrics()
	if failures.Load() != 0 {
		panic(fmt.Sprintf("echo failures: %d", failures.Load()))
	}
}
