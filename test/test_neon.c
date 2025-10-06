#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <pthread.h>
#include "project_neon.h"

// Client callbacks
void on_pong(uint64_t response_time_ms, uint64_t timestamp) {
    printf("[Client Callback] Pong received! RTT: %lu ms, Timestamp: %lu\n", 
           response_time_ms, timestamp);
}

void on_session_config(uint8_t version, uint16_t tick_rate, uint16_t max_packet_size) {
    printf("[Client Callback] Session Config - Version: %u, Tick Rate: %u Hz, Max Packet Size: %u bytes\n",
           version, tick_rate, max_packet_size);
}

void on_packet_type_registry(size_t count, const uint8_t* ids, const char** names, const char** descriptions) {
    printf("[Client Callback] Packet Type Registry received with %zu types:\n", count);
    for (size_t i = 0; i < count; i++) {
        printf("  [%u] %s - %s\n", ids[i], names[i], descriptions[i]);
    }
}

void on_unhandled_packet(uint8_t packet_type, uint8_t from_client_id) {
    printf("[Client Callback] Unhandled packet type %u from client %u\n", 
           packet_type, from_client_id);
}

void on_wrong_destination(uint8_t my_id, uint8_t packet_destination_id) {
    printf("[Client Callback] Wrong destination! My ID: %u, Packet for: %u\n",
           my_id, packet_destination_id);
}

// Host callbacks
void on_client_connect(uint8_t client_id, const char* name, uint32_t session_id) {
    printf("[Host Callback] Client connected! ID: %u, Name: %s, Session: %u\n",
           client_id, name, session_id);
}

void on_client_deny(const char* name, const char* reason) {
    printf("[Host Callback] Client denied! Name: %s, Reason: %s\n",
           name, reason);
}

void on_ping_received(uint8_t from_client_id) {
    printf("[Host Callback] Ping received from client %u\n", from_client_id);
}

void on_host_unhandled_packet(uint8_t packet_type, uint8_t from_client_id) {
    printf("[Host Callback] Unhandled packet type %u from client %u\n",
           packet_type, from_client_id);
}

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
    
    printf("=== Project Neon Callback Test ===\n");
    printf("Make sure relay is running at %s\n\n", relay_addr);
    
    // Create and configure host
    printf("[Main] Creating host for session %u...\n", session_id);
    NeonHostHandle* host = neon_host_new(session_id, relay_addr);
    if (!host) {
        printf("[Main] Failed to create host\n");
        const char* err = neon_get_last_error();
        if (err) printf("[Main] Error: %s\n", err);
        return 1;
    }
    printf("[Main] Host created successfully\n");
    
    // Register all host callbacks
    printf("[Main] Registering host callbacks...\n");
    neon_host_set_client_connect_callback(host, on_client_connect);
    neon_host_set_client_deny_callback(host, on_client_deny);
    neon_host_set_ping_received_callback(host, on_ping_received);
    neon_host_set_unhandled_packet_callback(host, on_host_unhandled_packet);
    
    // Start host in separate thread
    pthread_t host_thread;
    if (pthread_create(&host_thread, NULL, host_thread_func, host) != 0) {
        printf("[Main] Failed to create host thread\n");
        neon_host_free(host);
        return 1;
    }
    
    printf("[Main] Waiting for host to register...\n");
    sleep(2);
    
    // Create and configure clients
    printf("\n[Main] Creating clients...\n");
    NeonClientHandle* client1 = neon_client_new("TestClient1");
    NeonClientHandle* client2 = neon_client_new("TestClient2");
    
    if (!client1 || !client2) {
        printf("[Main] Failed to create clients\n");
        const char* err = neon_get_last_error();
        if (err) printf("[Main] Error: %s\n", err);
        return 1;
    }
    printf("[Main] Clients created\n");
    
    // Register all client callbacks for both clients
    printf("[Main] Registering client 1 callbacks...\n");
    neon_client_set_pong_callback(client1, on_pong);
    neon_client_set_session_config_callback(client1, on_session_config);
    neon_client_set_packet_type_registry_callback(client1, on_packet_type_registry);
    neon_client_set_unhandled_packet_callback(client1, on_unhandled_packet);
    neon_client_set_wrong_destination_callback(client1, on_wrong_destination);
    
    printf("[Main] Registering client 2 callbacks...\n");
    neon_client_set_pong_callback(client2, on_pong);
    neon_client_set_session_config_callback(client2, on_session_config);
    neon_client_set_packet_type_registry_callback(client2, on_packet_type_registry);
    neon_client_set_unhandled_packet_callback(client2, on_unhandled_packet);
    neon_client_set_wrong_destination_callback(client2, on_wrong_destination);
    
    // Connect client 1
    printf("\n[Main] Connecting client 1...\n");
    if (neon_client_connect(client1, session_id, relay_addr)) {
        printf("[Main] Client 1 connected! ID: %u\n", neon_client_get_id(client1));
    } else {
        printf("[Main] Client 1 failed to connect\n");
        const char* err = neon_get_last_error();
        if (err) printf("[Main] Error: %s\n", err);
    }
    
    sleep(1);
    
    // Connect client 2
    printf("\n[Main] Connecting client 2...\n");
    if (neon_client_connect(client2, session_id, relay_addr)) {
        printf("[Main] Client 2 connected! ID: %u\n", neon_client_get_id(client2));
    } else {
        printf("[Main] Client 2 failed to connect\n");
        const char* err = neon_get_last_error();
        if (err) printf("[Main] Error: %s\n", err);
    }
    
    sleep(1);
    printf("\n[Main] Host has %zu connected clients\n", neon_host_get_client_count(host));
    
    // Test manual pings
    printf("\n[Main] Testing manual ping from client 1...\n");
    if (neon_client_send_ping(client1)) {
        printf("[Main] Ping sent from client 1\n");
    } else {
        printf("[Main] Failed to send ping from client 1\n");
    }
    
    sleep(1);
    
    printf("\n[Main] Testing manual ping from client 2...\n");
    if (neon_client_send_ping(client2)) {
        printf("[Main] Ping sent from client 2\n");
    } else {
        printf("[Main] Failed to send ping from client 2\n");
    }
    
    // Run main processing loop
    printf("\n[Main] Running clients for 15 seconds...\n");
    printf("[Main] Auto-ping is enabled by default (every 5 seconds)\n\n");
    
    for (int i = 0; i < 150; i++) {
        if (neon_client_is_connected(client1)) {
            if (!neon_client_process_packets(client1)) {
                printf("[Main] Client 1 process_packets failed\n");
            }
        }
        if (neon_client_is_connected(client2)) {
            if (!neon_client_process_packets(client2)) {
                printf("[Main] Client 2 process_packets failed\n");
            }
        }
        usleep(100000); // 100ms
        
        if (i % 10 == 0) {
            printf("[Main] Tick %d - Client1 ID: %u (Connected: %d), Client2 ID: %u (Connected: %d)\n", 
                   i/10, 
                   neon_client_get_id(client1),
                   neon_client_is_connected(client1),
                   neon_client_get_id(client2),
                   neon_client_is_connected(client2));
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