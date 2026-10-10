/**
 * @file gdb_decoder.cpp
 * @brief On-the-fly human-readable GDB Remote Serial Protocol packet decoder (requests and replies).
 */

#include "gdb_decoder.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cctype>

// ANSI color escape codes for terminal output
#define ANSI_CYAN    "\033[36m"
#define ANSI_YELLOW  "\033[33m"
#define ANSI_GREEN   "\033[32m"
#define ANSI_MAGENTA "\033[35m"
#define ANSI_BLUE    "\033[34m"
#define ANSI_RED     "\033[31m"
#define ANSI_RESET   "\033[0m"

static bool s_decoder_enabled = false;

// Packet buffer size
static constexpr size_t GDB_PACKET_BUFFER_MAX = 4096;

// Request parser stream state
static char s_req_buffer[GDB_PACKET_BUFFER_MAX];
static size_t s_req_index = 0;
enum class ReqParserState { IDLE, IN_PACKET, CHECKSUM1, CHECKSUM2 };
static ReqParserState s_req_state = ReqParserState::IDLE;

// Reply parser stream state
static char s_rep_buffer[GDB_PACKET_BUFFER_MAX];
static size_t s_rep_index = 0;
enum class RepParserState { IDLE, IN_PACKET, CHECKSUM1, CHECKSUM2 };
static RepParserState s_rep_state = RepParserState::IDLE;

// Command context tracking to correlate replies with requests
enum class LastCommandType {
    NONE,
    MEM_READ,
    MEM_WRITE,
    REG_READ,
    ALL_REGS_READ,
    CONTINUE,
    STEP,
    HALT_QUERY,
    MONITOR,
    OTHER
};

struct CommandContext {
    LastCommandType type = LastCommandType::NONE;
    unsigned long long addr = 0;
    unsigned long length = 0;
    unsigned int reg_num = 0;
    char mon_cmd[128] = {0};
};

static CommandContext s_last_cmd;

void gdb_decoder_set_enabled(bool enabled)
{
    s_decoder_enabled = enabled;
    if (s_decoder_enabled) {
        printf(ANSI_GREEN "[GDB Decoder] On-the-fly GDB packet & reply decoding ENABLED" ANSI_RESET "\n");
    }
}

bool gdb_decoder_is_enabled()
{
    return s_decoder_enabled;
}

static uint8_t hex_char_to_val(char c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return 0;
}

/**
 * @brief Decode hex-encoded ASCII string.
 */
static void decode_hex_ascii(const char *hex_str, char *out_str, size_t max_out)
{
    size_t in_len = strlen(hex_str);
    size_t out_idx = 0;
    for (size_t i = 0; i + 1 < in_len && out_idx + 1 < max_out; i += 2) {
        uint8_t val = (hex_char_to_val(hex_str[i]) << 4) | hex_char_to_val(hex_str[i + 1]);
        out_str[out_idx++] = static_cast<char>(val);
    }
    out_str[out_idx] = '\0';
}

/**
 * @brief Convert target little-endian hex bytes to uint32_t.
 */
static uint32_t parse_target_le_u32(const char *hex_str)
{
    if (strlen(hex_str) < 8) return 0;
    uint32_t val = 0;
    for (int i = 0; i < 4; ++i) {
        uint8_t b = (hex_char_to_val(hex_str[i * 2]) << 4) | hex_char_to_val(hex_str[i * 2 + 1]);
        val |= (static_cast<uint32_t>(b) << (i * 8));
    }
    return val;
}

void gdb_decode_request_packet(const char *packet, size_t len)
{
    if (len == 0) return;

    char cmd = packet[0];
    const char *args = &packet[1];

    switch (cmd) {
    case 'm': { // Memory read: m<addr>,<length>
        unsigned long long addr = 0;
        unsigned long length = 0;
        if (sscanf(args, "%llx,%lx", &addr, &length) == 2) {
            s_last_cmd.type = LastCommandType::MEM_READ;
            s_last_cmd.addr = addr;
            s_last_cmd.length = length;
            printf(ANSI_CYAN "[GDB Req] Memory READ: addr=0x%llx, len=%lu bytes (%lu words)" ANSI_RESET "\n",
                   addr, length, (length + 3) / 4);
        } else {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_CYAN "[GDB Req] Memory READ (raw: %s)" ANSI_RESET "\n", packet);
        }
        break;
    }
    case 'M': { // Memory write: M<addr>,<length>:<hexdata>
        unsigned long long addr = 0;
        unsigned long length = 0;
        s_last_cmd.type = LastCommandType::MEM_WRITE;
        if (sscanf(args, "%llx,%lx", &addr, &length) == 2) {
            s_last_cmd.addr = addr;
            s_last_cmd.length = length;
            const char *colon = strchr(args, ':');
            const char *data_preview = colon ? colon + 1 : "";
            printf(ANSI_YELLOW "[GDB Req] Memory WRITE: addr=0x%llx, len=%lu bytes (preview: %.16s...)" ANSI_RESET "\n",
                   addr, length, data_preview);
        } else {
            printf(ANSI_YELLOW "[GDB Req] Memory WRITE (raw: %s)" ANSI_RESET "\n", packet);
        }
        break;
    }
    case 'X': { // Binary memory write: X<addr>,<length>:<data>
        unsigned long long addr = 0;
        unsigned long length = 0;
        s_last_cmd.type = LastCommandType::MEM_WRITE;
        if (sscanf(args, "%llx,%lx", &addr, &length) == 2) {
            s_last_cmd.addr = addr;
            s_last_cmd.length = length;
            printf(ANSI_YELLOW "[GDB Req] Binary memory WRITE: addr=0x%llx, len=%lu bytes" ANSI_RESET "\n",
                   addr, length);
        } else {
            printf(ANSI_YELLOW "[GDB Req] Binary memory WRITE (raw: %s)" ANSI_RESET "\n", packet);
        }
        break;
    }
    case 'g': // Read general registers
        s_last_cmd.type = LastCommandType::ALL_REGS_READ;
        printf(ANSI_CYAN "[GDB Req] Read all general registers" ANSI_RESET "\n");
        break;
    case 'G': // Write general registers
        s_last_cmd.type = LastCommandType::OTHER;
        printf(ANSI_YELLOW "[GDB Req] Write all general registers (%zu hex chars)" ANSI_RESET "\n", len - 1);
        break;
    case 'p': { // Read single register: p<reg>
        unsigned int reg_num = 0;
        s_last_cmd.type = LastCommandType::REG_READ;
        if (sscanf(args, "%x", &reg_num) == 1) {
            s_last_cmd.reg_num = reg_num;
            printf(ANSI_CYAN "[GDB Req] Read register #%u (0x%x)" ANSI_RESET "\n", reg_num, reg_num);
        } else {
            printf(ANSI_CYAN "[GDB Req] Read register (raw: %s)" ANSI_RESET "\n", packet);
        }
        break;
    }
    case 'P': { // Write single register: P<reg>=<val>
        unsigned int reg_num = 0;
        char val_str[64] = {0};
        s_last_cmd.type = LastCommandType::OTHER;
        if (sscanf(args, "%x=%63s", &reg_num, val_str) == 2) {
            printf(ANSI_YELLOW "[GDB Req] Write register #%u = 0x%s" ANSI_RESET "\n", reg_num, val_str);
        } else {
            printf(ANSI_YELLOW "[GDB Req] Write register (raw: %s)" ANSI_RESET "\n", packet);
        }
        break;
    }
    case 'c': { // Continue
        s_last_cmd.type = LastCommandType::CONTINUE;
        if (len > 1) {
            printf(ANSI_GREEN "[GDB Req] Continue execution at 0x%s" ANSI_RESET "\n", args);
        } else {
            printf(ANSI_GREEN "[GDB Req] Continue execution" ANSI_RESET "\n");
        }
        break;
    }
    case 's': { // Step
        s_last_cmd.type = LastCommandType::STEP;
        if (len > 1) {
            printf(ANSI_GREEN "[GDB Req] Single-step at 0x%s" ANSI_RESET "\n", args);
        } else {
            printf(ANSI_GREEN "[GDB Req] Single-step" ANSI_RESET "\n");
        }
        break;
    }
    case '?': // Halt status query
        s_last_cmd.type = LastCommandType::HALT_QUERY;
        printf(ANSI_MAGENTA "[GDB Req] Query target halt reason / stop status" ANSI_RESET "\n");
        break;
    case 'k': // Kill
        s_last_cmd.type = LastCommandType::OTHER;
        printf(ANSI_YELLOW "[GDB Req] Kill target" ANSI_RESET "\n");
        break;
    case 'D': // Detach
        s_last_cmd.type = LastCommandType::OTHER;
        printf(ANSI_MAGENTA "[GDB Req] Detach from target" ANSI_RESET "\n");
        break;
    case 'H': { // Set thread: H<op><thread>
        s_last_cmd.type = LastCommandType::OTHER;
        char op = args[0];
        const char *op_str = (op == 'c') ? "continue/step" : (op == 'g') ? "other operations" : "unknown";
        printf(ANSI_MAGENTA "[GDB Req] Set thread for %s: thread=%s" ANSI_RESET "\n", op_str, &args[1]);
        break;
    }
    case 'T': // Thread alive query
        s_last_cmd.type = LastCommandType::OTHER;
        printf(ANSI_MAGENTA "[GDB Req] Check if thread %s is alive" ANSI_RESET "\n", args);
        break;
    case 'Z':
    case 'z': { // Breakpoints / Watchpoints
        s_last_cmd.type = LastCommandType::OTHER;
        bool insert = (cmd == 'Z');
        int type = -1;
        unsigned long long addr = 0;
        unsigned int kind = 0;
        if (sscanf(args, "%d,%llx,%x", &type, &addr, &kind) >= 2) {
            const char *type_name = "unknown";
            switch (type) {
            case 0: type_name = "software breakpoint"; break;
            case 1: type_name = "hardware breakpoint"; break;
            case 2: type_name = "write watchpoint"; break;
            case 3: type_name = "read watchpoint"; break;
            case 4: type_name = "access watchpoint"; break;
            }
            printf(ANSI_YELLOW "[GDB Req] %s %s at 0x%llx (kind=%u)" ANSI_RESET "\n",
                   insert ? "Insert" : "Remove", type_name, addr, kind);
        } else {
            printf(ANSI_YELLOW "[GDB Req] Breakpoint %s (raw: %s)" ANSI_RESET "\n",
                   insert ? "Insert" : "Remove", packet);
        }
        break;
    }
    case 'v': { // Extended 'v' packets
        if (strncmp(packet, "vCont?", 6) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query supported vCont actions" ANSI_RESET "\n");
        } else if (strncmp(packet, "vCont;", 6) == 0) {
            s_last_cmd.type = (packet[6] == 's') ? LastCommandType::STEP : LastCommandType::CONTINUE;
            printf(ANSI_GREEN "[GDB Req] Action vCont: %s" ANSI_RESET "\n", packet + 6);
        } else if (strncmp(packet, "vAttach;", 8) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Attach to PID: %s" ANSI_RESET "\n", packet + 8);
        } else {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Extended packet: %s" ANSI_RESET "\n", packet);
        }
        break;
    }
    case 'q': { // Query packets
        if (strncmp(packet, "qRcmd,", 6) == 0) {
            s_last_cmd.type = LastCommandType::MONITOR;
            decode_hex_ascii(packet + 6, s_last_cmd.mon_cmd, sizeof(s_last_cmd.mon_cmd));
            printf(ANSI_GREEN "[GDB Req] Monitor command: \"mon %s\"" ANSI_RESET "\n", s_last_cmd.mon_cmd);
        } else if (strncmp(packet, "qSupported", 10) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query supported features: %s" ANSI_RESET "\n", packet + 10);
        } else if (strncmp(packet, "qXfer:memory-map:read:", 22) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_CYAN "[GDB Req] Read target memory-map XML" ANSI_RESET "\n");
        } else if (strncmp(packet, "qXfer:features:read:", 20) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_CYAN "[GDB Req] Read target description XML (%s)" ANSI_RESET "\n", packet + 20);
        } else if (strcmp(packet, "qAttached") == 0 || strncmp(packet, "qAttached:", 10) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query attached status (%s)" ANSI_RESET "\n", packet);
        } else if (strcmp(packet, "qOffsets") == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query section offsets" ANSI_RESET "\n");
        } else if (strcmp(packet, "qC") == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query current thread ID" ANSI_RESET "\n");
        } else if (strncmp(packet, "qfThreadInfo", 12) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query thread list (first)" ANSI_RESET "\n");
        } else if (strncmp(packet, "qsThreadInfo", 12) == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query thread list (subsequent)" ANSI_RESET "\n");
        } else if (strcmp(packet, "qTStatus") == 0) {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query trace status" ANSI_RESET "\n");
        } else {
            s_last_cmd.type = LastCommandType::OTHER;
            printf(ANSI_MAGENTA "[GDB Req] Query: %s" ANSI_RESET "\n", packet);
        }
        break;
    }
    default:
        s_last_cmd.type = LastCommandType::OTHER;
        printf("[GDB Req] Packet: $%s\n", packet);
        break;
    }
}

void gdb_decode_reply_packet(const char *packet, size_t len)
{
    if (len == 0) {
        printf(ANSI_YELLOW "  [GDB Rep] <unsupported / empty reply>" ANSI_RESET "\n");
        return;
    }

    // Success response
    if (strcmp(packet, "OK") == 0) {
        printf(ANSI_GREEN "  [GDB Rep] OK (success)" ANSI_RESET "\n");
        return;
    }

    // Error response: E<nn>
    if (packet[0] == 'E' && len == 3 && isxdigit(packet[1]) && isxdigit(packet[2])) {
        printf(ANSI_RED "  [GDB Rep] ERROR code %s" ANSI_RESET "\n", packet);
        return;
    }

    // Halt / Stop response: T<sig><thread_info...> or S<sig>
    if (packet[0] == 'T' || packet[0] == 'S') {
        unsigned int sig = 0;
        if (sscanf(packet + 1, "%02x", &sig) == 1) {
            const char *sig_name = "unknown";
            switch (sig) {
            case 0x02: sig_name = "SIGINT (interrupted)"; break;
            case 0x05: sig_name = "SIGTRAP (breakpoint / step reached)"; break;
            case 0x0b: sig_name = "SIGSEGV (access fault)"; break;
            }
            const char *info = (packet[0] == 'T' && len > 3) ? packet + 3 : "";
            printf(ANSI_MAGENTA "  [GDB Rep] Target HALTED: %s (sig 0x%02x%s%s)" ANSI_RESET "\n",
                   sig_name, sig, info[0] ? ", " : "", info);
            return;
        }
    }

    // Console output response: O<hex_ascii>
    if (packet[0] == 'O') {
        char console_text[512] = {0};
        decode_hex_ascii(packet + 1, console_text, sizeof(console_text));
        printf(ANSI_GREEN "  [GDB Console] %s" ANSI_RESET, console_text);
        if (console_text[strlen(console_text) - 1] != '\n') {
            printf("\n");
        }
        return;
    }

    // Contextual decode based on the last command
    switch (s_last_cmd.type) {
    case LastCommandType::MEM_READ: {
        // Reply contains hex string of read bytes
        size_t byte_count = len / 2;
        if (byte_count <= 8) {
            // Short read: display all bytes and 32-bit word if 4 bytes
            if (byte_count == 4) {
                uint32_t w = parse_target_le_u32(packet);
                printf(ANSI_BLUE "  [GDB Rep] Memory READ data (addr=0x%llx): 0x%08x (hex: %s)" ANSI_RESET "\n",
                       s_last_cmd.addr, w, packet);
            } else {
                printf(ANSI_BLUE "  [GDB Rep] Memory READ data (addr=0x%llx, %zu bytes): %s" ANSI_RESET "\n",
                       s_last_cmd.addr, byte_count, packet);
            }
        } else {
            // Longer read: preview first 16 bytes
            printf(ANSI_BLUE "  [GDB Rep] Memory READ data (addr=0x%llx, %zu bytes): %.32s..." ANSI_RESET "\n",
                   s_last_cmd.addr, byte_count, packet);
        }
        break;
    }

    case LastCommandType::REG_READ: {
        // Reply contains 32-bit register value in target endian
        uint32_t reg_val = parse_target_le_u32(packet);
        printf(ANSI_BLUE "  [GDB Rep] Register #%u value: 0x%08x (raw: %s)" ANSI_RESET "\n",
               s_last_cmd.reg_num, reg_val, packet);
        break;
    }

    case LastCommandType::ALL_REGS_READ: {
        printf(ANSI_BLUE "  [GDB Rep] All general registers dump (%zu bytes hex)" ANSI_RESET "\n", len);
        break;
    }

    case LastCommandType::MONITOR: {
        printf(ANSI_BLUE "  [GDB Rep] Monitor reply: %s" ANSI_RESET "\n", packet);
        break;
    }

    default:
        printf(ANSI_BLUE "  [GDB Rep] Data (%zu bytes): %.64s%s" ANSI_RESET "\n",
               len, packet, (len > 64) ? "..." : "");
        break;
    }
}

void gdb_decode_incoming(const uint8_t *data, uint32_t len)
{
    if (!s_decoder_enabled || !data || len == 0) {
        return;
    }

    for (uint32_t i = 0; i < len; ++i) {
        char c = static_cast<char>(data[i]);

        switch (s_req_state) {
        case ReqParserState::IDLE:
            if (c == '$') {
                s_req_index = 0;
                s_req_state = ReqParserState::IN_PACKET;
            } else if (c == 0x03) { // Ctrl-C
                printf(ANSI_YELLOW "[GDB Req] Break / Interrupt request (0x03)" ANSI_RESET "\n");
            } else if (c == '+') {
                // Client ACK (silent)
            } else if (c == '-') {
                printf(ANSI_YELLOW "[GDB Req] NACK received (-)" ANSI_RESET "\n");
            }
            break;

        case ReqParserState::IN_PACKET:
            if (c == '#') {
                s_req_state = ReqParserState::CHECKSUM1;
            } else {
                if (s_req_index + 1 < GDB_PACKET_BUFFER_MAX) {
                    s_req_buffer[s_req_index++] = c;
                }
            }
            break;

        case ReqParserState::CHECKSUM1:
            s_req_state = ReqParserState::CHECKSUM2;
            break;

        case ReqParserState::CHECKSUM2:
            s_req_buffer[s_req_index] = '\0';
            gdb_decode_request_packet(s_req_buffer, s_req_index);
            s_req_index = 0;
            s_req_state = ReqParserState::IDLE;
            break;
        }
    }
}

void gdb_decode_outgoing(const uint8_t *data, uint32_t len)
{
    if (!s_decoder_enabled || !data || len == 0) {
        return;
    }

    for (uint32_t i = 0; i < len; ++i) {
        char c = static_cast<char>(data[i]);

        switch (s_rep_state) {
        case RepParserState::IDLE:
            if (c == '$') {
                s_rep_index = 0;
                s_rep_state = RepParserState::IN_PACKET;
            } else if (c == '+') {
                // Server ACK
            } else if (c == '-') {
                printf(ANSI_YELLOW "  [GDB Rep] NACK (-)" ANSI_RESET "\n");
            }
            break;

        case RepParserState::IN_PACKET:
            if (c == '#') {
                s_rep_state = RepParserState::CHECKSUM1;
            } else {
                if (s_rep_index + 1 < GDB_PACKET_BUFFER_MAX) {
                    s_rep_buffer[s_rep_index++] = c;
                }
            }
            break;

        case RepParserState::CHECKSUM1:
            s_rep_state = RepParserState::CHECKSUM2;
            break;

        case RepParserState::CHECKSUM2:
            s_rep_buffer[s_rep_index] = '\0';
            gdb_decode_reply_packet(s_rep_buffer, s_rep_index);
            s_rep_index = 0;
            s_rep_state = RepParserState::IDLE;
            break;
        }
    }
}
