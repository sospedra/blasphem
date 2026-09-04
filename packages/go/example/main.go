// Command example judges two messages with packs read from a directory:
//
//	go run ./example ../../resources/packs
package main

import (
	"fmt"
	"os"

	blasphem "github.com/sospedra/blasphem/packages/go"
)

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: example <packs directory>")
		os.Exit(2)
	}
	err := blasphem.Init(blasphem.Options{Locales: []string{"en", "es"}, Assets: os.Args[1], Grawlix: true})
	if err != nil {
		fmt.Fprintln(os.Stderr, "init:", err)
		os.Exit(1)
	}
	defer blasphem.Close()
	fmt.Printf("%+v\n", blasphem.Judge("you are a stupid loser"))
	fmt.Printf("%+v\n", blasphem.Judge("물이 별로 없다."))
	_, err = blasphem.New(blasphem.Options{Locales: []string{"xx"}, Assets: os.Args[1]})
	fmt.Println("error:", err)
}
