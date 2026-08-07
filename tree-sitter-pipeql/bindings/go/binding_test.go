package tree_sitter_pipeql_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_pipeql "github.com/tree-sitter/tree-sitter-pipeql/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_pipeql.Language())
	if language == nil {
		t.Errorf("Error loading Pipeql grammar")
	}
}
