CC = gcc
CFLAGS = -Wall -Wextra -g -std=c11
LDFLAGS = -lcriterion

all: tests

tests: my_tests.c
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

clean:
	rm -f tests
