package pipeql

import (
	"fmt"
	"strings"
	"testing"
)

var readShapes = []string{
	"from orders | join customers on orders.customer_id == customers.id | filter orders.status == 'active' and orders.total >= $min_total | group [customer_id, region] (total = sum(orders.total), cnt = count(*)) | filter total > $threshold | select [customer_id, region, total, cnt] | sort [total desc, customer_id asc] | take 10",
	"from events | filter event_type in ['click', 'view', 'hover'] and user_id is not null | group [user_id, day] (views = count(*), last = max(timestamp)) | sort [views desc] | take 100 | select [user_id, day, views, last]",
	"from users u | join orders o on u.id == o.user_id | join payments p on o.id == p.order_id | filter u.age >= {low} and u.age <= {high} and p.amount > 0 | select [u.id, u.name, o.total, p.amount] | sort [o.total desc] | take 10",
	"from log_entries | filter level in ['error', 'warn'] and not (service == 'health') | group [service, date] (errors = count(*)) | filter errors > $min_errors | select [service, date, errors] | sort [date desc, errors desc] | take 50",
	"from products | filter category == 'electronics' | filter price >= $min_price and price <= $max_price | select [id, name, price] | sort [price asc] | take 20 | skip 40",
	"from sessions s | join users u on s.user_id == u.id | filter u.plan in ['pro', 'enterprise'] and s.duration >= $min_seconds | group [u.id] (total_duration = sum(s.duration), sessions = count(*)) | select [u.id, u.name, total_duration, sessions] | sort [total_duration desc] | take 10",
	"from inventory | filter stock <= $reorder_point and (warehouse == 'north' or warehouse == 'east') | select [sku, name, stock, warehouse] | sort [stock asc, warehouse asc]",
	"from transactions t | join accounts a on t.account_id == a.id | filter t.status == 'completed' and a.balance >= $min_balance | group [a.country] (revenue = sum(t.amount), count = count(*)) | filter count >= $min_count | select [a.country, revenue, count] | sort [revenue desc] | take 100",
}

var mutationShapes = []string{
	"into notes | insert [title = $title, content = $content, category = 'Personal', is_pinned = 0, created_at = CURRENT_TIMESTAMP]",
	"from notes | filter is_archived == 0 | filter id == $id | update [title = $title, is_pinned = 1, updated_at = CURRENT_TIMESTAMP]",
	"from notes | filter id == $id or is_archived == 1 | delete",
}

func buildCorpus(n int) []string {
	allShapes := make([]string, 0, len(readShapes)+len(mutationShapes))
	allShapes = append(allShapes, readShapes...)
	allShapes = append(allShapes, mutationShapes...)

	queries := make([]string, n)
	for i := 0; i < n; i++ {
		shape := allShapes[i%len(allShapes)]
		if strings.Contains(shape, "{low}") {
			low := fmt.Sprintf("%d", i)
			high := fmt.Sprintf("%d", i+40)
			queries[i] = strings.Replace(strings.Replace(shape, "{low}", low, 1), "{high}", high, 1)
		} else {
			queries[i] = shape
		}
	}
	return queries
}

func buildMutationCorpus(n int) []string {
	queries := make([]string, n)
	for i := 0; i < n; i++ {
		queries[i] = mutationShapes[i%len(mutationShapes)]
	}
	return queries
}

func BenchmarkReadCorpus1000(b *testing.B) {
	queries := buildCorpus(1000)
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, q := range queries {
			_, err := Compile(q, "postgres")
			if err != nil {
				b.Fatal(err)
			}
		}
	}
}

func BenchmarkMutationCorpus1000(b *testing.B) {
	queries := buildMutationCorpus(1000)
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, q := range queries {
			_, err := Compile(q, "postgres")
			if err != nil {
				b.Fatal(err)
			}
		}
	}
}

func BenchmarkSingleLargest(b *testing.B) {
	q := readShapes[0]
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := Compile(q, "postgres")
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkSingleLargestMutation(b *testing.B) {
	q := mutationShapes[0]
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := Compile(q, "postgres")
		if err != nil {
			b.Fatal(err)
		}
	}
}
