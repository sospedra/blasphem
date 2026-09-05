// Command example judges messages with two embedded locales:
//
//	go run ./example
package main

import (
	"encoding/json"
	"fmt"
	"os"

	blasphem "github.com/sospedra/blasphem/packages/go/v2"
	"github.com/sospedra/blasphem/packages/go/v2/locales/en"
	"github.com/sospedra/blasphem/packages/go/v2/locales/es"
)

func main() {
	err := blasphem.Init(blasphem.Options{Locales: []blasphem.Locale{en.Locale, es.Locale}, Grawlix: true})
	if err != nil {
		fmt.Fprintln(os.Stderr, "init:", err)
		os.Exit(1)
	}
	defer blasphem.Close()
	printJudgement("you are a stupid loser")
	printJudgement("물이 별로 없다.")
	_, err = blasphem.New(blasphem.Options{LocaleCodes: []string{"xx"}})
	fmt.Println("error:", err)
}

func printJudgement(text string) {
	if err := json.NewEncoder(os.Stdout).Encode(blasphem.Judge(text)); err != nil {
		fmt.Fprintln(os.Stderr, "encode:", err)
		os.Exit(1)
	}
}
