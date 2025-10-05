/**
 * Project Neon - UDP Networking Library
 * C API for integration with Unreal Engine and other C/C++ applications
 */

#ifndef PROJECT_NEON_H
#define PROJECT_NEON_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NeonClientHandle NeonClientHandle;

/**
 * Create a new Neon client
 * @param name Client name (null-terminated string)
 * @return Client handle, or NULL on failure
 */
NeonClientHandle* neon_client_new(const char* name);

/**
 * Connect the client to a session through a relay
 * @param client Client handle
 * @param session_id Session ID to connect to
 * @param relay_addr Relay address (e.g. "127.0.0.1:7777")
 * @return true on success, false on failure
 */
bool neon_client_connect(
    NeonClientHandle* client,
    uint32_t session_id,
    const char* relay_addr
);

/**
 * Process incoming packets
 * Call this regularly in your game loop (e.g. every tick/frame)
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
 * @param client Client handle
 * @param enabled true to enable auto-ping, false to disable
 */
void neon_client_set_auto_ping(NeonClientHandle* client, bool enabled);

/**
 * Free the client and release resources
 * @param client Client handle
 */
void neon_client_free(NeonClientHandle* client);

typedef struct NeonHostHandle NeonHostHandle;

/**
 * Create a new Neon host
 * @param session_id Session ID for this host
 * @param relay_addr Relay address (e.g. "127.0.0.1:7777")
 * @return Host handle, or NULL on failure
 */
NeonHostHandle* neon_host_new(uint32_t session_id, const char* relay_addr);

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