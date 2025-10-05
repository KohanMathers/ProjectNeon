CC = gcc
CFLAGS = -Wall -I.

ifeq ($(OS),Windows_NT)
    LIB = project_neon.dll
    LDFLAGS = -L. -lproject_neon -lpthread
else
    LIB = libproject_neon.so
    LDFLAGS = -L. -lproject_neon -lpthread -Wl,-rpath,.
endif

TARGET = test_neon
SRC = test_neon.c

all: $(TARGET)

$(TARGET): $(SRC) $(LIB)
	$(CC) $(CFLAGS) -o $(TARGET) $(SRC) $(LDFLAGS)

clean:
	rm -f $(TARGET)

run: $(TARGET)
	./$(TARGET)

.PHONY: all clean run