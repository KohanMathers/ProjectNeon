#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <pthread.h>
#include "project_neon.h"

void* host_thread_func(void* arg) {
    NeonHostHandle* host = (NeonHostHandle*)arg;
    printf("[Host Thread] Starting host...\n");
    
    if (!neon_host_start(host)) {
        printf("[Host Thread] Failed to start host\n");
        const char* err = neon_get_last_error();
        if (err) printf("[Host Thread] Error: %s\n", err);
    }
    
    return NULL;
}

int main() {
    const char* relay_addr = "127.0.0.1:7777";
    uint32_t session_id = 12345;
    
    printf("=== Project Neon Test ===\n");
    printf("Make sure relay is running at %s\n\n", relay_addr);
    
    printf("[Main] Creating host for session %u...\n", session_id);
    NeonHostHandle* host = neon_host_new(session_id, relay_addr);
    if (!host) {
        printf("[Main] Failed to create host\n");
        return 1;
    }
    printf("[Main] Host created successfully\n");
    
    pthread_t host_thread;
    if (pthread_create(&host_thread, NULL, host_thread_func, host) != 0) {
        printf("[Main] Failed to create host thread\n");
        neon_host_free(host);
        return 1;
    }
    
    printf("[Main] Waiting for host to register...\n");
    sleep(2);
    
    printf("\n[Main] Creating clients...\n");
    NeonClientHandle* client1 = neon_client_new("TestClient1");
    NeonClientHandle* client2 = neon_client_new("TestClient2");
    
    if (!client1 || !client2) {
        printf("[Main] Failed to create clients\n");
        return 1;
    }
    printf("[Main] Clients created\n");
    
    printf("\n[Main] Connecting client 1...\n");
    if (neon_client_connect(client1, session_id, relay_addr)) {
        printf("[Main] Client 1 connected! ID: %u\n", neon_client_get_id(client1));
    } else {
        printf("[Main] Client 1 failed to connect\n");
    }
    
    sleep(1);
    
    printf("\n[Main] Connecting client 2...\n");
    if (neon_client_connect(client2, session_id, relay_addr)) {
        printf("[Main] Client 2 connected! ID: %u\n", neon_client_get_id(client2));
    } else {
        printf("[Main] Client 2 failed to connect\n");
    }
    
    sleep(1);
    printf("\n[Main] Host has %zu connected clients\n", neon_host_get_client_count(host));
    
    printf("\n[Main] Running clients for 10 seconds...\n");
    for (int i = 0; i < 100; i++) {
        if (neon_client_is_connected(client1)) {
            neon_client_process_packets(client1);
        }
        if (neon_client_is_connected(client2)) {
            neon_client_process_packets(client2);
        }
        usleep(100000);
        
        if (i % 10 == 0) {
            printf("[Main] Tick %d - Client1 ID: %u, Client2 ID: %u\n", 
                   i/10, 
                   neon_client_get_id(client1),
                   neon_client_get_id(client2));
        }
    }
    
    printf("\n[Main] Cleaning up...\n");
    neon_client_free(client1);
    neon_client_free(client2);
    
    printf("[Main] Test complete!\n");
    printf("[Main] Press Ctrl+C to exit (host thread still running)\n");
    
    pthread_join(host_thread, NULL);
    neon_host_free(host);
    
    return 0;
}