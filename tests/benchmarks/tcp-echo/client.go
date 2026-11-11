package main

import (
	"bufio"
	"bytes"
	"encoding/binary"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"math/bits"
	"net"
	"os"
	"runtime"
	"strconv"
	"sync"
	"time"
)

// Logarithmic buckets have at most 1/64 relative width, with exact values below 64 ns.
type sample struct {
	histogram [4096]uint64
	measured  uint64
	total     uint64
	sumNS     uint64
	maxNS     uint64
	last      time.Time
	err       string
}

func bucket(value uint64) int {
	if value < 64 {
		return int(value)
	}
	exponent := bits.Len64(value) - 1
	return 64 + (exponent-6)*64 + int((value-(uint64(1)<<exponent))>>(exponent-6))
}

func upperBound(index int) uint64 {
	if index < 64 {
		return uint64(index)
	}
	exponent := (index-64)/64 + 6
	return (uint64(1) << exponent) + uint64((index-64)%64+1)*(uint64(1)<<(exponent-6)) - 1
}

func writeAll(peer net.Conn, data []byte) error {
	for len(data) != 0 {
		n, err := peer.Write(data)
		if err != nil {
			return err
		}
		if n == 0 {
			return io.ErrShortWrite
		}
		data = data[n:]
	}
	return nil
}

func main() {
	address := flag.String("address", "127.0.0.1:18082", "echo server")
	shards := flag.Int("shards", 1, "consecutive listening ports")
	connections := flag.Int("connections", 64, "persistent connections")
	payload := flag.Int("payload", 1024, "bytes per round trip, at least eight")
	seconds := flag.Float64("seconds", 6, "measured seconds")
	warmup := flag.Float64("warmup", 1, "warmup seconds on the same connections")
	workers := flag.Int("workers", 8, "client GOMAXPROCS")
	flag.Parse()
	if *connections < 1 || *payload < 8 || *seconds <= 0 || *warmup < 0 || *workers < 1 {
		panic("invalid client configuration")
	}
	runtime.GOMAXPROCS(*workers)
	host, portText, err := net.SplitHostPort(*address)
	if err != nil {
		panic(err)
	}
	port, err := strconv.Atoi(portText)
	if err != nil || *shards < 1 {
		panic("invalid shard address")
	}
	peers := make([]net.Conn, *connections)
	for i := range peers {
		peer, err := net.DialTimeout("tcp4", net.JoinHostPort(host, strconv.Itoa(port+i%*shards)), 10*time.Second)
		if err != nil {
			panic(err)
		}
		if err := peer.(*net.TCPConn).SetNoDelay(true); err != nil {
			panic(err)
		}
		peers[i] = peer
		defer peer.Close()
	}
	input := bufio.NewScanner(os.Stdin)
	fmt.Println("READY")
	if !input.Scan() || input.Text() != "RUN" {
		panic("expected RUN")
	}
	begin := time.Now()
	measuredBegin := begin.Add(time.Duration(*warmup * float64(time.Second)))
	end := measuredBegin.Add(time.Duration(*seconds * float64(time.Second)))
	results := make([]sample, *connections)
	var group sync.WaitGroup
	for index, peer := range peers {
		if err := peer.SetDeadline(end.Add(10 * time.Second)); err != nil {
			panic(err)
		}
		group.Add(1)
		go func(index int, peer net.Conn) {
			defer group.Done()
			outgoing, incoming := make([]byte, *payload), make([]byte, *payload)
			for i := range outgoing {
				outgoing[i] = byte(i*31 + index)
			}
			result := &results[index]
			for {
				start := time.Now()
				if !start.Before(end) {
					break
				}
				binary.LittleEndian.PutUint64(outgoing[:8], result.total)
				if err := writeAll(peer, outgoing); err != nil {
					result.err = err.Error()
					return
				}
				if _, err := io.ReadFull(peer, incoming); err != nil {
					result.err = err.Error()
					return
				}
				finished := time.Now()
				if !bytes.Equal(outgoing, incoming) {
					result.err = "echo content mismatch"
					return
				}
				result.total++
				if !start.Before(measuredBegin) {
					latency := uint64(finished.Sub(start))
					result.histogram[bucket(latency)]++
					result.measured++
					result.sumNS += latency
					if latency > result.maxNS {
						result.maxNS = latency
					}
					result.last = finished
				}
			}
		}(index, peer)
	}
	time.Sleep(time.Until(measuredBegin))
	fmt.Println("MEASURE_START")
	group.Wait()
	fmt.Println("MEASURE_END")
	var merged sample
	var errors []string
	last := end
	for _, result := range results {
		merged.measured += result.measured
		merged.total += result.total
		merged.sumNS += result.sumNS
		if result.maxNS > merged.maxNS {
			merged.maxNS = result.maxNS
		}
		if result.last.After(last) {
			last = result.last
		}
		if result.err != "" {
			errors = append(errors, result.err)
		}
		for index, count := range result.histogram {
			merged.histogram[index] += count
		}
	}
	percentile := func(fraction float64) float64 {
		target := uint64(float64(merged.measured)*fraction + 0.999999)
		var count uint64
		for index, value := range merged.histogram {
			count += value
			if count >= target {
				return float64(upperBound(index)) / 1000
			}
		}
		return 0
	}
	duration := last.Sub(measuredBegin).Seconds()
	result := map[string]any{
		"requests": merged.measured, "requests_with_warmup": merged.total,
		"duration_s": duration, "qps": float64(merged.measured) / duration,
		"payload_bytes": *payload, "connections": *connections,
		"p50_us": percentile(0.5), "p90_us": percentile(0.9),
		"p99_us": percentile(0.99), "p999_us": percentile(0.999),
		"max_us": float64(merged.maxNS) / 1000, "errors": errors,
	}
	if merged.measured > 0 {
		result["mean_us"] = float64(merged.sumNS) / float64(merged.measured) / 1000
	}
	encoded, err := json.Marshal(result)
	if err != nil {
		panic(err)
	}
	fmt.Println("RESULT", string(encoded))
	if !input.Scan() || input.Text() != "CLOSE" {
		panic("expected CLOSE")
	}
}
