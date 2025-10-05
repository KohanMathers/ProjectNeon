CC = gcc
CFLAGS = -Wall -I.
LDFLAGS = -L. -lproject_neon -lpthread -Wl,-rpath,.

TARGET = test_neon
SRC = test_neon.c
LIB = libproject_neon.so

all: $(TARGET)

$(TARGET): $(SRC) $(LIB)
	$(CC) $(CFLAGS) -o $(TARGET) $(SRC) $(LDFLAGS)

clean:
	rm -f $(TARGET)

run: $(TARGET)
	./$(TARGET)

.PHONY: all clean run