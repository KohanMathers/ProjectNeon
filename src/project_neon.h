#ifndef PROJECT_NEON_H
#define PROJECT_NEON_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NeonClientHandle NeonClientHandle;
typedef struct NeonHostHandle NeonHostHandle;

/**
 * Called when a pong response is received
 * @param response_time_ms Round-trip time in milliseconds
 * @param timestamp Current timestamp when pong was received
 */
typedef void (*PongCallback)(uint64_t response_time_ms, uint64_t timestamp);

/**
 * Called when session configuration is received from the host
 * @param version Protocol version
 * @param tick_rate Server tick rate (Hz)
 * @param max_packet_size Maximum packet size in bytes
 */
typedef void (*SessionConfigCallback)(uint8_t version, uint16_t tick_rate, uint16_t max_packet_size);

/**
 * Called when packet type registry is received from the host
 * @param count Number of packet types in the registry
 * @param ids Array of packet IDs (length = count)
 * @param names Array of packet names as null-terminated strings (length = count)
 * @param descriptions Array of packet descriptions as null-terminated strings (length = count)
 */
typedef void (*PacketTypeRegistryCallback)(size_t count, const uint8_t* ids, const char** names, const char** descriptions);

/**
 * Called when an unhandled/unknown packet type is received
 * @param packet_type The type ID of the unhandled packet
 * @param from_client_id Client ID that sent the packet
 */
typedef void (*UnhandledPacketCallback)(uint8_t packet_type, uint8_t from_client_id);

/**
 * Called when a packet is received that's addressed to the wrong destination
 * @param my_id This client's ID
 * @param packet_destination_id The destination ID specified in the packet header
 */
typedef void (*WrongDestinationCallback)(uint8_t my_id, uint8_t packet_destination_id);

/**
 * Called when a client successfully connects to the session
 * @param client_id The assigned client ID
 * @param name The client's name (null-terminated string)
 * @param session_id The session ID they connected to
 */
typedef void (*ClientConnectCallback)(uint8_t client_id, const char* name, uint32_t session_id);

/**
 * Called when a client connection is denied
 * @param name The client's name that was denied (null-terminated string)
 * @param reason The reason for denial (null-terminated string)
 */
typedef void (*ClientDenyCallback)(const char* name, const char* reason);

/**
 * Called when a ping packet is received from a client
 * @param from_client_id The client ID that sent the ping
 */
typedef void (*PingReceivedCallback)(uint8_t from_client_id);

/**
 * Called when the host receives an unhandled/unknown packet type
 * @param packet_type The type ID of the unhandled packet
 * @param from_client_id Client ID that sent the packet
 */
typedef void (*HostUnhandledPacketCallback)(uint8_t packet_type, uint8_t from_client_id);

/**
 * Create a new Neon client
 * @param name Client name (null-terminated string)
 * @return Client handle, or NULL on failure
 */
NeonClientHandle* neon_client_new(const char* name);

/**
 * Set callback for pong events
 * Call this before connecting to receive pong notifications
 * @param client Client handle
 * @param callback Callback function pointer
 */
void neon_client_set_pong_callback(NeonClientHandle* client, PongCallback callback);

/**
 * Set callback for session config events
 * Call this before connecting to receive session configuration
 * @param client Client handle
 * @param callback Callback function pointer
 */
void neon_client_set_session_config_callback(NeonClientHandle* client, SessionConfigCallback callback);

/**
 * Set callback for packet type registry events
 * Call this before connecting to receive packet type information
 * @param client Client handle
 * @param callback Callback function pointer
 */
void neon_client_set_packet_type_registry_callback(NeonClientHandle* client, PacketTypeRegistryCallback callback);

/**
 * Set callback for unhandled packet events
 * @param client Client handle
 * @param callback Callback function pointer
 */
void neon_client_set_unhandled_packet_callback(NeonClientHandle* client, UnhandledPacketCallback callback);

/**
 * Set callback for wrong destination events
 * @param client Client handle
 * @param callback Callback function pointer
 */
void neon_client_set_wrong_destination_callback(NeonClientHandle* client, WrongDestinationCallback callback);

/**
 * Connect the client to a session through a relay
 * @param client Client handle
 * @param session_id Session ID to connect to
 * @param relay_addr Relay address (e.g. "127.0.0.1:7777")
 * @return true on success, false on failure
 */
bool neon_client_connect(NeonClientHandle* client, uint32_t session_id, const char* relay_addr);

/**
 * Process incoming packets
 * Call this regularly in your game loop (e.g. every tick/frame)
 * This will trigger any registered callbacks when events occur
 * @param client Client handle
 * @return true on success, false on failure
 */
bool neon_client_process_packets(NeonClientHandle* client);

/**
 * Get the client's assigned ID
 * @param client Client handle
 * @return Client ID, or 0 if not connected
 */
uint8_t neon_client_get_id(NeonClientHandle* client);

/**
 * Get the session ID
 * @param client Client handle
 * @return Session ID, or 0 if not connected
 */
uint32_t neon_client_get_session_id(NeonClientHandle* client);

/**
 * Check if the client is connected
 * @param client Client handle
 * @return true if connected, false otherwise
 */
bool neon_client_is_connected(NeonClientHandle* client);

/**
 * Manually send a ping packet
 * @param client Client handle
 * @return true on success, false on failure
 */
bool neon_client_send_ping(NeonClientHandle* client);

/**
 * Enable or disable automatic pinging
 * When enabled (default), the client automatically sends pings every 5 seconds
 * @param client Client handle
 * @param enabled true to enable auto-ping, false to disable
 */
void neon_client_set_auto_ping(NeonClientHandle* client, bool enabled);

/**
 * Free the client and release resources
 * @param client Client handle
 */
void neon_client_free(NeonClientHandle* client);

/**
 * Create a new Neon host
 * @param session_id Session ID for this host
 * @param relay_addr Relay address (e.g. "127.0.0.1:7777")
 * @return Host handle, or NULL on failure
 */
NeonHostHandle* neon_host_new(uint32_t session_id, const char* relay_addr);

/**
 * Set callback for client connect events
 * @param host Host handle
 * @param callback Callback function pointer
 */
void neon_host_set_client_connect_callback(NeonHostHandle* host, ClientConnectCallback callback);

/**
 * Set callback for client deny events
 * @param host Host handle
 * @param callback Callback function pointer
 */
void neon_host_set_client_deny_callback(NeonHostHandle* host, ClientDenyCallback callback);

/**
 * Set callback for ping received events
 * @param host Host handle
 * @param callback Callback function pointer
 */
void neon_host_set_ping_received_callback(NeonHostHandle* host, PingReceivedCallback callback);

/**
 * Set callback for unhandled packet events
 * @param host Host handle
 * @param callback Callback function pointer
 */
void neon_host_set_unhandled_packet_callback(NeonHostHandle* host, HostUnhandledPacketCallback callback);

/**
 * Get the host's session ID
 * @param host Host handle
 * @return Session ID
 */
uint32_t neon_host_get_session_id(NeonHostHandle* host);

/**
 * Get the number of connected clients
 * @param host Host handle
 * @return Number of connected clients
 */
size_t neon_host_get_client_count(NeonHostHandle* host);

/**
 * Start the host (BLOCKING CALL - run in a separate thread!)
 * This function will block until an error occurs
 * Callbacks will be triggered as events occur
 * @param host Host handle
 * @return true on success, false on failure
 */
bool neon_host_start(NeonHostHandle* host);

/**
 * Free the host and release resources
 * @param host Host handle
 */
void neon_host_free(NeonHostHandle* host);

/**
 * Get the last error message
 * @return Error message, or NULL if no error
 * Note: The returned string is valid until the next error or thread exit
 */
const char* neon_get_last_error(void);

#ifdef __cplusplus
}
#endif

#endif // PROJECT_NEON_H