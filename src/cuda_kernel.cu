#include "cuda_compat.h"
/* C compatibility For dumb IDEs: */
#ifndef __OPENCL_VERSION__
#ifndef __cplusplus
typedef int bool;
#endif
typedef unsigned char uchar;
typedef unsigned short ushort;
typedef unsigned int uint;
typedef unsigned long ulong;
typedef unsigned long size_t;
typedef long ptrdiff_t;
typedef size_t uintptr_t;
typedef ptrdiff_t intptr_t;
#ifndef __kernel
#define __kernel
#endif
#ifndef __global
#define __global
#endif
#ifndef __private
#define __private
#endif
#ifndef __local
#define __local
#endif
#ifndef __constant
#define __constant const
#endif
#endif /* __OPENCL_VERSION__ */

#define ARGON2_D  0
#define ARGON2_I  1
#define ARGON2_ID 2

#define ARGON2_VERSION_10 0x10
#define ARGON2_VERSION_13 0x13

#define ARGON2_BLOCK_SIZE 1024
#define ARGON2_QWORDS_IN_BLOCK (ARGON2_BLOCK_SIZE / 8)
#define ARGON2_SYNC_POINTS 4

#define THREADS_PER_LANE 32
#define QWORDS_PER_THREAD (ARGON2_QWORDS_IN_BLOCK / 32)

__constant ulong BLAKE2B_IV[8] = {
    0x6a09e667f3bcc908UL, 0xbb67ae8584caa73bUL,
    0x3c6ef372fe94f82bUL, 0xa54ff53a5f1d36f1UL,
    0x510e527fade682d1UL, 0x9b05688c2b3e6c1fUL,
    0x1f83d9abfb41bd6bUL, 0x5be0cd19137e2179UL
};

__constant uchar BLAKE2B_SIGMA[12][16] = {
    { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15 },
    {14,10, 4, 8, 9,15,13, 6, 1,12, 0, 2,11, 7, 5, 3 },
    {11, 8,12, 0, 5, 2,15,13,10,14, 3, 6, 7, 1, 9, 4 },
    { 7, 9, 3, 1,13,12,11,14, 2, 6, 5,10, 4, 0,15, 8 },
    { 9, 0, 5, 7, 2, 4,10,15,14, 1,11,12, 6, 8, 3,13 },
    { 2,12, 6,10, 0,11, 8, 3, 4,13, 7, 5,15,14, 1, 9 },
    {12, 5, 1,15,14,13, 4,10, 0, 7, 6, 3, 9, 2, 8,11 },
    {13,11, 7,14,12, 1, 3, 9, 5, 0,15, 4, 8, 6, 2,10 },
    { 6,15,14, 9,11, 3, 0, 8,12, 2,13, 7, 1, 4,10, 5 },
    {10, 2, 8, 4, 7, 6, 1, 5,15,11, 9,14, 3,12,13, 0 },
    { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15 },
    {14,10, 4, 8, 9,15,13, 6, 1,12, 0, 2,11, 7, 5, 3 }
};

__constant ulong POW_INITIAL_STATE[25] = {
    1242148031264380989UL, 3008272977830772284UL, 2188519011337848018UL, 1992179434288343456UL, 8876506674959887717UL,
    5399642050693751366UL, 1745875063082670864UL, 8605242046444978844UL, 17936695144567157056UL, 3343109343542796272UL,
    1123092876221303306UL, 4963925045340115282UL, 17037383077651887893UL, 16629644495023626889UL, 12833675776649114147UL,
    3784524041015224902UL, 1082795874807940378UL, 13952716920571277634UL, 13411128033953605860UL, 15060696040649351053UL,
    9928834659948351306UL, 5237849264682708699UL, 12825353012139217522UL, 6706187291358897596UL, 196324915476054915UL
};

__constant ulong HEAVY_INITIAL_STATE[25] = {
    4239941492252378377UL, 8746723911537738262UL, 8796936657246353646UL, 1272090201925444760UL, 16654558671554924250UL,
    8270816933120786537UL, 13907396207649043898UL, 6782861118970774626UL, 9239690602118867528UL, 11582319943599406348UL,
    17596056728278508070UL, 15212962468105129023UL, 7812475424661425213UL, 3370482334374859748UL, 5690099369266491460UL,
    8596393687355028144UL, 570094237299545110UL, 9119540418498120711UL, 16901969272480492857UL, 13372017233735502424UL,
    14372891883993151831UL, 5171152063242093102UL, 10573107899694386186UL, 6096431547456407061UL, 1592359455985097269UL
};

__constant ulong KECCAKF_RNDC[24] = {
    0x0000000000000001UL, 0x0000000000008082UL, 0x800000000000808aUL, 0x8000000080008000UL,
    0x000000000000808bUL, 0x0000000080000001UL, 0x8000000080008081UL, 0x8000000000008009UL,
    0x000000000000008aUL, 0x0000000000000088UL, 0x0000000080008009UL, 0x000000008000000aUL,
    0x000000008000808bUL, 0x800000000000008bUL, 0x8000000000008089UL, 0x8000000000008003UL,
    0x8000000000008002UL, 0x8000000000000080UL, 0x000000000000800aUL, 0x800000008000000aUL,
    0x8000000080008081UL, 0x8000000000008080UL, 0x0000000080000001UL, 0x8000000080008008UL
};

__constant uint KECCAKF_ROTC[24] = {
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14,
    27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44
};

__constant uint KECCAKF_PILN[24] = {
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4,
    15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1
};

__device__ __forceinline__ ulong b2b_rotr64(ulong x, uint n)
{
    return (x >> n) | (x << (64 - n));
}

__device__ __forceinline__ ulong rotl64(ulong x, uint n)
{
    return (x << n) | (x >> (64 - n));
}

__device__ __forceinline__ ulong load64_le_private(__private const uchar *src)
{
    ulong v = 0;
    for (uint i = 0; i < 8; i++) {
        v |= ((ulong)src[i]) << (8 * i);
    }
    return v;
}

__device__ __forceinline__ ulong load64_le_global(__global const uchar *src)
{
    ulong v = 0;
    for (uint i = 0; i < 8; i++) {
        v |= ((ulong)src[i]) << (8 * i);
    }
    return v;
}

__device__ __forceinline__ void store32_le_private(__private uchar *dst, uint v)
{
    dst[0] = (uchar)(v);
    dst[1] = (uchar)(v >> 8);
    dst[2] = (uchar)(v >> 16);
    dst[3] = (uchar)(v >> 24);
}

__device__ __forceinline__ void store64_le_private(__private uchar *dst, ulong v)
{
    for (uint i = 0; i < 8; i++) {
        dst[i] = (uchar)(v >> (8 * i));
    }
}

__device__ __forceinline__ void store64_le_global(__global uchar *dst, ulong v)
{
    for (uint i = 0; i < 8; i++) {
        dst[i] = (uchar)(v >> (8 * i));
    }
}

__device__ __forceinline__ void keccak_f1600_state(__private ulong *st)
{
    ulong bc[5];
    ulong t;

    for (uint round = 0; round < 24; round++) {
        for (uint i = 0; i < 5; i++) {
            bc[i] = st[i] ^ st[i + 5] ^ st[i + 10] ^ st[i + 15] ^ st[i + 20];
        }

        for (uint i = 0; i < 5; i++) {
            t = bc[(i + 4) % 5] ^ rotl64(bc[(i + 1) % 5], 1);
            for (uint j = 0; j < 25; j += 5) {
                st[j + i] ^= t;
            }
        }

        t = st[1];
        for (uint i = 0; i < 24; i++) {
            uint j = KECCAKF_PILN[i];
            bc[0] = st[j];
            st[j] = rotl64(t, KECCAKF_ROTC[i]);
            t = bc[0];
        }

        for (uint j = 0; j < 25; j += 5) {
            for (uint i = 0; i < 5; i++) {
                bc[i] = st[j + i];
            }
            for (uint i = 0; i < 5; i++) {
                st[j + i] ^= (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
            }
        }

        st[0] ^= KECCAKF_RNDC[round];
    }
}

__device__ __forceinline__ void store_u256_private(__private uchar *dst, __private const ulong *words)
{
    for (uint i = 0; i < 4; i++) {
        store64_le_private(dst + i * 8, words[i]);
    }
}

__device__ __forceinline__ ushort load_matrix_value(__global const uchar *constants, uint row, uint col)
{
    uint offset = 72 + (row * 64 + col) * 2;
    return (ushort)constants[offset] | ((ushort)constants[offset + 1] << 8);
}

__device__ __forceinline__ void heavy_hash_words(__private ulong *out, __private const ulong *in_words)
{
    ulong state[25];
    for (uint i = 0; i < 25; i++) {
        state[i] = HEAVY_INITIAL_STATE[i];
    }
    for (uint i = 0; i < 4; i++) {
        state[i] ^= in_words[i];
    }
    keccak_f1600_state(state);
    for (uint i = 0; i < 4; i++) {
        out[i] = state[i];
    }
}

__device__ __forceinline__ void matrix_heavy_hash_words(__private ulong *out, __global const uchar *constants, __private const ulong *input_words)
{
    uchar input[32];
    uchar nibbles[64];
    uchar product[32];
    ulong product_words[4];

    store_u256_private(input, input_words);
    for (uint i = 0; i < 32; i++) {
        nibbles[2 * i] = input[i] >> 4;
        nibbles[2 * i + 1] = input[i] & 0x0f;
    }

    for (uint i = 0; i < 32; i++) {
        uint sum1 = 0;
        uint sum2 = 0;
        for (uint j = 0; j < 64; j++) {
            uchar elem = nibbles[j];
            sum1 += (uint)load_matrix_value(constants, 2 * i, j) * (uint)elem;
            sum2 += (uint)load_matrix_value(constants, 2 * i + 1, j) * (uint)elem;
        }
        product[i] = (uchar)(((sum1 >> 10) << 4) | (sum2 >> 10));
        product[i] ^= input[i];
    }

    for (uint i = 0; i < 4; i++) {
        product_words[i] = load64_le_private(product + i * 8);
    }
    heavy_hash_words(out, product_words);
}

#define B2B_G(a,b,c,d,x,y) \
    do { \
        v[a] = v[a] + v[b] + (x); \
        v[d] = b2b_rotr64(v[d] ^ v[a], 32); \
        v[c] = v[c] + v[d]; \
        v[b] = b2b_rotr64(v[b] ^ v[c], 24); \
        v[a] = v[a] + v[b] + (y); \
        v[d] = b2b_rotr64(v[d] ^ v[a], 16); \
        v[c] = v[c] + v[d]; \
        v[b] = b2b_rotr64(v[b] ^ v[c], 63); \
    } while (0)

__device__ __forceinline__ void blake2b_compress_private(__private ulong *h, __private const uchar *block, ulong t, bool last)
{
    ulong m[16];
    ulong v[16];

    for (uint i = 0; i < 16; i++) {
        m[i] = load64_le_private(block + i * 8);
    }

    for (uint i = 0; i < 8; i++) {
        v[i] = h[i];
        v[i + 8] = BLAKE2B_IV[i];
    }
    v[12] ^= t;
    if (last) {
        v[14] = ~v[14];
    }

    for (uint r = 0; r < 12; r++) {
        B2B_G(0, 4,  8, 12, m[BLAKE2B_SIGMA[r][ 0]], m[BLAKE2B_SIGMA[r][ 1]]);
        B2B_G(1, 5,  9, 13, m[BLAKE2B_SIGMA[r][ 2]], m[BLAKE2B_SIGMA[r][ 3]]);
        B2B_G(2, 6, 10, 14, m[BLAKE2B_SIGMA[r][ 4]], m[BLAKE2B_SIGMA[r][ 5]]);
        B2B_G(3, 7, 11, 15, m[BLAKE2B_SIGMA[r][ 6]], m[BLAKE2B_SIGMA[r][ 7]]);
        B2B_G(0, 5, 10, 15, m[BLAKE2B_SIGMA[r][ 8]], m[BLAKE2B_SIGMA[r][ 9]]);
        B2B_G(1, 6, 11, 12, m[BLAKE2B_SIGMA[r][10]], m[BLAKE2B_SIGMA[r][11]]);
        B2B_G(2, 7,  8, 13, m[BLAKE2B_SIGMA[r][12]], m[BLAKE2B_SIGMA[r][13]]);
        B2B_G(3, 4,  9, 14, m[BLAKE2B_SIGMA[r][14]], m[BLAKE2B_SIGMA[r][15]]);
    }

    for (uint i = 0; i < 8; i++) {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

__device__ __forceinline__ void blake2b_hash_private(__private uchar *out, uint out_len, __private const uchar *input, uint input_len)
{
    ulong h[8];
    uchar block[128];

    for (uint i = 0; i < 8; i++) {
        h[i] = BLAKE2B_IV[i];
    }
    h[0] ^= 0x01010000UL ^ (ulong)out_len;

    uint offset = 0;
    if (input_len > 128) {
        for (uint i = 0; i < 128; i++) {
            block[i] = input[i];
        }
        blake2b_compress_private(h, block, 128, false);
        offset = 128;
    }

    uint remaining = input_len - offset;
    for (uint i = 0; i < 128; i++) {
        block[i] = i < remaining ? input[offset + i] : 0;
    }
    blake2b_compress_private(h, block, (ulong)input_len, true);

    for (uint i = 0; i < out_len; i++) {
        out[i] = (uchar)(h[i >> 3] >> (8 * (i & 7)));
    }
}

__device__ __forceinline__ void blake2b_hash32_prefix_block_global(__private uchar *out, __global const uchar *input)
{
    ulong h[8];
    uchar block[128];
    const uint input_len = 1024;
    const uint total_len = 4 + input_len;

    for (uint i = 0; i < 8; i++) {
        h[i] = BLAKE2B_IV[i];
    }
    h[0] ^= 0x01010000UL ^ 32UL;

    store32_le_private(block, 32);
    for (uint i = 0; i < 124; i++) {
        block[4 + i] = input[i];
    }
    blake2b_compress_private(h, block, 128, false);

    uint consumed_input = 124;
    uint compressed_total = 128;
    while (total_len - compressed_total > 128) {
        for (uint i = 0; i < 128; i++) {
            block[i] = input[consumed_input + i];
        }
        consumed_input += 128;
        compressed_total += 128;
        blake2b_compress_private(h, block, compressed_total, false);
    }

    uint remaining = total_len - compressed_total;
    for (uint i = 0; i < 128; i++) {
        block[i] = i < remaining ? input[consumed_input + i] : 0;
    }
    blake2b_compress_private(h, block, total_len, true);

    for (uint i = 0; i < 32; i++) {
        out[i] = (uchar)(h[i >> 3] >> (8 * (i & 7)));
    }
}

__device__ __forceinline__ void blake2b_keyed_pow_72(__private uchar *out, __private const uchar *input)
{
    ulong h[8];
    uchar block[128];

    for (uint i = 0; i < 8; i++) {
        h[i] = BLAKE2B_IV[i];
    }
    h[0] ^= 0x01010000UL ^ (15UL << 8) ^ 32UL;

    for (uint i = 0; i < 128; i++) {
        block[i] = 0;
    }
    block[0] = 'P'; block[1] = 'r'; block[2] = 'o'; block[3] = 'o';
    block[4] = 'f'; block[5] = 'O'; block[6] = 'f'; block[7] = 'W';
    block[8] = 'o'; block[9] = 'r'; block[10] = 'k'; block[11] = 'H';
    block[12] = 'a'; block[13] = 's'; block[14] = 'h';
    blake2b_compress_private(h, block, 128, false);

    for (uint i = 0; i < 128; i++) {
        block[i] = i < 72 ? input[i] : 0;
    }
    blake2b_compress_private(h, block, 128 + 72, true);

    for (uint i = 0; i < 32; i++) {
        out[i] = (uchar)(h[i >> 3] >> (8 * (i & 7)));
    }
}

__device__ __forceinline__ void digest_long_1024_global(__global uchar *out, __private const uchar *input72)
{
    uchar data[128];
    uchar buffer[64];
    uchar next[64];

    store32_le_private(data, 1024);
    for (uint i = 0; i < 72; i++) {
        data[4 + i] = input72[i];
    }
    blake2b_hash_private(buffer, 64, data, 76);

    for (uint i = 0; i < 32; i++) {
        out[i] = buffer[i];
    }
    uint produced = 32;

    while (1024 - produced > 64) {
        blake2b_hash_private(next, 64, buffer, 64);
        for (uint i = 0; i < 64; i++) {
            buffer[i] = next[i];
        }
        for (uint i = 0; i < 32; i++) {
            out[produced + i] = buffer[i];
        }
        produced += 32;
    }

    blake2b_hash_private(next, 64, buffer, 64);
    for (uint i = 0; i < 64; i++) {
        out[produced + i] = next[i];
    }
}

__kernel void velkar_prepare_jobs(
        __global uchar *job_inputs,
        __global const uchar *constants,
        ulong input_per_job,
        ulong start_nonce,
        ulong nonce_mask,
        ulong nonce_fixed,
        ulong target0,
        ulong target1,
        ulong target2,
        ulong target3)
{
    ulong job = get_global_id(0);
    ulong nonce = ((start_nonce + job) & nonce_mask) | nonce_fixed;
    __global uchar *out = job_inputs + job * input_per_job;

    ulong pre_pow[4];
    ulong state[25];
    ulong stage1[4];
    ulong stage2[4];
    ulong stage3[4];
    ulong targets[4] = { target0, target1, target2, target3 };
    uchar stage1_bytes[32];
    uchar stage3_bytes[32];

    for (uint i = 0; i < 4; i++) {
        pre_pow[i] = load64_le_global(constants + i * 8);
    }
    ulong timestamp = load64_le_global(constants + 32);

    for (uint i = 0; i < 25; i++) {
        state[i] = POW_INITIAL_STATE[i];
    }
    for (uint i = 0; i < 4; i++) {
        state[i] ^= pre_pow[i];
    }
    state[4] ^= timestamp;
    state[9] ^= nonce;
    keccak_f1600_state(state);
    for (uint i = 0; i < 4; i++) {
        stage1[i] = state[i];
    }

    matrix_heavy_hash_words(stage2, constants, stage1);
    heavy_hash_words(stage3, stage2);

    store_u256_private(stage1_bytes, stage1);
    store_u256_private(stage3_bytes, stage3);

    for (uint i = 0; i < 32; i++) {
        out[i] = stage3_bytes[i];
    }
    for (uint i = 0; i < 4; i++) {
        for (uint b = 0; b < 8; b++) {
            out[32 + i * 8 + b] = (uchar)(targets[i] >> (8 * b));
        }
    }
    store64_le_global(out + 64, nonce);

    for (uint i = 0; i < 16; i++) {
        out[72 + i] = stage1_bytes[i] ^ stage3_bytes[i + 16];
    }
    store64_le_global(out + 88, nonce);
    for (uint i = 0; i < 32; i++) {
        out[96 + i] = stage1_bytes[i];
    }
    for (uint i = 0; i < 4; i++) {
        for (uint b = 0; b < 8; b++) {
            out[128 + i * 8 + b] = (uchar)(stage2[i] >> (8 * b));
            out[160 + i * 8 + b] = (uchar)(stage3[i] >> (8 * b));
        }
    }
}

__kernel void velkar_finalize_candidates(
        __global const uchar *memory,
        __global const uchar *job_inputs,
        __global ulong *found_nonces,
        ulong memory_per_job,
        ulong input_per_job,
        ulong target0,
        ulong target1,
        ulong target2,
        ulong target3,
        ulong check0,
        ulong check1,
        ulong check2,
        ulong check3)
{
    ulong job = get_global_id(0);
    __global const uchar *input = job_inputs + job * input_per_job;
    __global const uchar *last_block = memory + job * memory_per_job + memory_per_job - ARGON2_BLOCK_SIZE;

    uchar stage4[32];
    uchar final_data[72];
    uchar stage5[32];
    blake2b_hash32_prefix_block_global(stage4, last_block);

    for (uint i = 0; i < 32; i++) {
        final_data[i] = stage4[i];
    }
    for (uint i = 0; i < 8; i++) {
        final_data[32 + i] = input[88 + i];
    }
    found_nonces[job] = 0UL;

    ulong targets[4] = { target0, target1, target2, target3 };
    for (uint w = 0; w < 4; w++) {
        for (uint b = 0; b < 8; b++) {
            final_data[40 + w * 8 + b] = (uchar)(targets[w] >> (8 * b));
        }
    }

    blake2b_keyed_pow_72(stage5, final_data);

    ulong words[4];
    for (uint w = 0; w < 4; w++) {
        words[w] = load64_le_private(stage5 + w * 8);
    }
    ulong nonce = load64_le_global(input + 88);
    ulong stage1_0 = load64_le_global(input + 96);
    ulong stage2_1 = load64_le_global(input + 128 + 8);
    ulong stage3_2 = load64_le_global(input + 160 + 16);
    ulong stage4_3 = load64_le_private(stage4 + 24);

    words[0] ^= rotl64(stage1_0, 17);
    words[1] ^= rotl64(stage2_1, 53);
    words[2] ^= rotl64(stage3_2, 7);
    words[3] ^= rotl64(stage4_3, 13) ^ rotl64(nonce, 13);

    bool pass = false;
    if (words[3] < check3) pass = true;
    else if (words[3] == check3) {
        if (words[2] < check2) pass = true;
        else if (words[2] == check2) {
            if (words[1] < check1) pass = true;
            else if (words[1] == check1 && words[0] <= check0) pass = true;
        }
    }

    if (pass) {
        found_nonces[job] = nonce;
    }
}

__kernel void velkar_export_stage4(
        __global const uchar *memory,
        __global const uchar *job_inputs,
        __global uchar *stage4_out,
        __global ulong *nonce_out,
        ulong memory_per_job,
        ulong input_per_job)
{
    ulong job = get_global_id(0);
    __global const uchar *input = job_inputs + job * input_per_job;
    __global const uchar *last_block = memory + job * memory_per_job + memory_per_job - ARGON2_BLOCK_SIZE;
    __global uchar *out = stage4_out + job * 32UL;

    uchar stage4[32];
    blake2b_hash32_prefix_block_global(stage4, last_block);

    for (uint i = 0; i < 32; i++) {
        out[i] = stage4[i];
    }
    nonce_out[job] = load64_le_global(input + 88);
}

__kernel void velkar_init_first_blocks(
        __global uchar *memory,
        __global const uchar *job_inputs,
        ulong memory_per_job,
        ulong input_per_job)
{
    ulong job = get_global_id(0);
    __global const uchar *input = job_inputs + job * input_per_job;
    __global uchar *dst = memory + job * memory_per_job;

    uchar init_data[128];
    uchar init_hash[72];

    store32_le_private(init_data + 0, 1);
    store32_le_private(init_data + 4, 32);
    store32_le_private(init_data + 8, 8192);
    store32_le_private(init_data + 12, 1);
    store32_le_private(init_data + 16, 0x13);
    store32_le_private(init_data + 20, 2);
    store32_le_private(init_data + 24, 72);
    for (uint i = 0; i < 72; i++) {
        init_data[28 + i] = input[i];
    }
    store32_le_private(init_data + 100, 16);
    for (uint i = 0; i < 16; i++) {
        init_data[104 + i] = input[72 + i];
    }
    store32_le_private(init_data + 120, 0);
    store32_le_private(init_data + 124, 0);

    blake2b_hash_private(init_hash, 64, init_data, 128);
    store32_le_private(init_hash + 68, 0);

    store32_le_private(init_hash + 64, 0);
    digest_long_1024_global(dst, init_hash);

    store32_le_private(init_hash + 64, 1);
    digest_long_1024_global(dst + 1024, init_hash);
}

__kernel void velkar_copy_initial_blocks(
        __global uchar *memory,
        __global const uchar *first_blocks,
        ulong memory_per_job,
        ulong first_blocks_per_job)
{
    ulong pos = get_global_id(0);
    ulong job = pos / first_blocks_per_job;
    ulong in_job = pos - job * first_blocks_per_job;
    memory[job * memory_per_job + in_job] = first_blocks[pos];
}

__kernel void velkar_copy_last_blocks(
        __global const uchar *memory,
        __global uchar *last_blocks,
        ulong memory_per_job,
        ulong last_block_bytes)
{
    ulong pos = get_global_id(0);
    ulong job = pos / last_block_bytes;
    ulong in_job = pos - job * last_block_bytes;
    last_blocks[pos] = memory[job * memory_per_job + memory_per_job - last_block_bytes + in_job];
}

#ifndef ARGON2_VERSION
#define ARGON2_VERSION ARGON2_VERSION_13
#endif

#ifndef ARGON2_TYPE
#define ARGON2_TYPE ARGON2_I
#endif

__device__ __forceinline__ ulong u64_build(uint hi, uint lo)
{
    return upsample(hi, lo);
}

__device__ __forceinline__ uint u64_lo(ulong x)
{
    return (uint)x;
}

__device__ __forceinline__ uint u64_hi(ulong x)
{
    return (uint)(x >> 32);
}

struct u64_shuffle_buf {
    uint lo[THREADS_PER_LANE];
    uint hi[THREADS_PER_LANE];
};

__device__ __forceinline__ ulong u64_shuffle(ulong v, uint thread_src, uint thread,
                  __local struct u64_shuffle_buf *buf)
{
#ifdef __CUDACC__
    (void)thread;
    (void)buf;
    uint lo = __shfl_sync(0xffffffffU, u64_lo(v), thread_src, THREADS_PER_LANE);
    uint hi = __shfl_sync(0xffffffffU, u64_hi(v), thread_src, THREADS_PER_LANE);
    return u64_build(hi, lo);
#else
    uint lo = u64_lo(v);
    uint hi = u64_hi(v);
    buf->lo[thread] = lo;
    buf->hi[thread] = hi;
    barrier(CLK_LOCAL_MEM_FENCE);
    lo = buf->lo[thread_src];
    hi = buf->hi[thread_src];
    barrier(CLK_LOCAL_MEM_FENCE);
    return u64_build(hi, lo);
#endif
}

struct block_g {
    ulong data[ARGON2_QWORDS_IN_BLOCK];
};

struct block_th {
    ulong a, b, c, d;
};

__device__ __forceinline__ ulong cmpeq_mask(uint test, uint ref)
{
    uint x = -(uint)(test == ref);
    return u64_build(x, x);
}

__device__ __forceinline__ ulong block_th_get(const struct block_th *b, uint idx)
{
    ulong res = 0;
    res ^= cmpeq_mask(idx, 0) & b->a;
    res ^= cmpeq_mask(idx, 1) & b->b;
    res ^= cmpeq_mask(idx, 2) & b->c;
    res ^= cmpeq_mask(idx, 3) & b->d;
    return res;
}

__device__ __forceinline__ void block_th_set(struct block_th *b, uint idx, ulong v)
{
    b->a ^= cmpeq_mask(idx, 0) & (v ^ b->a);
    b->b ^= cmpeq_mask(idx, 1) & (v ^ b->b);
    b->c ^= cmpeq_mask(idx, 2) & (v ^ b->c);
    b->d ^= cmpeq_mask(idx, 3) & (v ^ b->d);
}

__device__ __forceinline__ void move_block(struct block_th *dst, const struct block_th *src)
{
    *dst = *src;
}

__device__ __forceinline__ void xor_block(struct block_th *dst, const struct block_th *src)
{
    dst->a ^= src->a;
    dst->b ^= src->b;
    dst->c ^= src->c;
    dst->d ^= src->d;
}

__device__ __forceinline__ void load_block(struct block_th *dst, __global const struct block_g *src,
                uint thread)
{
    dst->a = src->data[0 * THREADS_PER_LANE + thread];
    dst->b = src->data[1 * THREADS_PER_LANE + thread];
    dst->c = src->data[2 * THREADS_PER_LANE + thread];
    dst->d = src->data[3 * THREADS_PER_LANE + thread];
}

__device__ __forceinline__ void load_block_xor(struct block_th *dst, __global const struct block_g *src,
                    uint thread)
{
    dst->a ^= src->data[0 * THREADS_PER_LANE + thread];
    dst->b ^= src->data[1 * THREADS_PER_LANE + thread];
    dst->c ^= src->data[2 * THREADS_PER_LANE + thread];
    dst->d ^= src->data[3 * THREADS_PER_LANE + thread];
}

__device__ __forceinline__ void store_block(__global struct block_g *dst, const struct block_th *src,
                 uint thread)
{
    dst->data[0 * THREADS_PER_LANE + thread] = src->a;
    dst->data[1 * THREADS_PER_LANE + thread] = src->b;
    dst->data[2 * THREADS_PER_LANE + thread] = src->c;
    dst->data[3 * THREADS_PER_LANE + thread] = src->d;
}

#if defined(cl_amd_media_ops) && !defined(VELKAR_DISABLE_AMD_MEDIA_OPS)
#pragma OPENCL EXTENSION cl_amd_media_ops : enable

__device__ __forceinline__ ulong rotr64(ulong x, ulong n)
{
    uint lo = u64_lo(x);
    uint hi = u64_hi(x);
    uint r_lo, r_hi;
    if (n < 32) {
        r_lo = amd_bitalign(hi, lo, (uint)n);
        r_hi = amd_bitalign(lo, hi, (uint)n);
    } else {
        r_lo = amd_bitalign(lo, hi, (uint)n - 32);
        r_hi = amd_bitalign(hi, lo, (uint)n - 32);
    }
    return u64_build(r_hi, r_lo);
}
#else
__device__ __forceinline__ ulong rotr64(ulong x, ulong n)
{
    return rotate(x, 64 - n);
}
#endif

__device__ __forceinline__ ulong f(ulong x, ulong y)
{
    uint xlo = u64_lo(x);
    uint ylo = u64_lo(y);
    return x + y + 2 * u64_build(mul_hi(xlo, ylo), xlo * ylo);
}

__device__ __forceinline__ void g(struct block_th *block)
{
    ulong a, b, c, d;
    a = block->a;
    b = block->b;
    c = block->c;
    d = block->d;

    a = f(a, b);
    d = rotr64(d ^ a, 32);
    c = f(c, d);
    b = rotr64(b ^ c, 24);
    a = f(a, b);
    d = rotr64(d ^ a, 16);
    c = f(c, d);
    b = rotr64(b ^ c, 63);

    block->a = a;
    block->b = b;
    block->c = c;
    block->d = d;
}

__device__ __forceinline__ uint apply_shuffle_shift1(uint thread, uint idx)
{
    return (thread & 0x1c) | ((thread + idx) & 0x3);
}

__device__ __forceinline__ uint apply_shuffle_unshift1(uint thread, uint idx)
{
    idx = (QWORDS_PER_THREAD - idx) % QWORDS_PER_THREAD;

    return apply_shuffle_shift1(thread, idx);
}

__device__ __forceinline__ uint apply_shuffle_shift2(uint thread, uint idx)
{
    uint lo = (thread & 0x1) | ((thread & 0x10) >> 3);
    lo = (lo + idx) & 0x3;
    return ((lo & 0x2) << 3) | (thread & 0xe) | (lo & 0x1);
}

__device__ __forceinline__ uint apply_shuffle_unshift2(uint thread, uint idx)
{
    idx = (QWORDS_PER_THREAD - idx) % QWORDS_PER_THREAD;

    return apply_shuffle_shift2(thread, idx);
}

__device__ __forceinline__ void shuffle_shift1(struct block_th *block, uint thread,
                    __local struct u64_shuffle_buf *buf)
{
    for (uint i = 0; i < QWORDS_PER_THREAD; i++) {
        uint src_thr = apply_shuffle_shift1(thread, i);

        ulong v = block_th_get(block, i);
        v = u64_shuffle(v, src_thr, thread, buf);
        block_th_set(block, i, v);
    }
}

__device__ __forceinline__ void shuffle_unshift1(struct block_th *block, uint thread,
                      __local struct u64_shuffle_buf *buf)
{
    for (uint i = 0; i < QWORDS_PER_THREAD; i++) {
        uint src_thr = apply_shuffle_unshift1(thread, i);

        ulong v = block_th_get(block, i);
        v = u64_shuffle(v, src_thr, thread, buf);
        block_th_set(block, i, v);
    }
}

__device__ __forceinline__ void shuffle_shift2(struct block_th *block, uint thread,
                    __local struct u64_shuffle_buf *buf)
{
    for (uint i = 0; i < QWORDS_PER_THREAD; i++) {
        uint src_thr = apply_shuffle_shift2(thread, i);

        ulong v = block_th_get(block, i);
        v = u64_shuffle(v, src_thr, thread, buf);
        block_th_set(block, i, v);
    }
}

__device__ __forceinline__ void shuffle_unshift2(struct block_th *block, uint thread,
                      __local struct u64_shuffle_buf *buf)
{
    for (uint i = 0; i < QWORDS_PER_THREAD; i++) {
        uint src_thr = apply_shuffle_unshift2(thread, i);

        ulong v = block_th_get(block, i);
        v = u64_shuffle(v, src_thr, thread, buf);
        block_th_set(block, i, v);
    }
}

__device__ __forceinline__ void transpose(struct block_th *block, uint thread,
               __local struct u64_shuffle_buf *buf)
{
    uint thread_group = (thread & 0x0C) >> 2;
    for (uint i = 1; i < QWORDS_PER_THREAD; i++) {
        uint thr = (i << 2) ^ thread;
        uint idx = thread_group ^ i;

        ulong v = block_th_get(block, idx);
        v = u64_shuffle(v, thr, thread, buf);
        block_th_set(block, idx, v);
    }
}

__device__ __forceinline__ void shuffle_block(struct block_th *block, uint thread,
                   __local struct u64_shuffle_buf *buf)
{
    transpose(block, thread, buf);

    g(block);

    shuffle_shift1(block, thread, buf);

    g(block);

    shuffle_unshift1(block, thread, buf);
    transpose(block, thread, buf);

    g(block);

    shuffle_shift2(block, thread, buf);

    g(block);

    shuffle_unshift2(block, thread, buf);
}

__device__ __forceinline__ void compute_ref_pos(uint lanes, uint segment_blocks,
                     uint pass, uint lane, uint slice, uint offset,
                     uint *ref_lane, uint *ref_index)
{
    uint lane_blocks = ARGON2_SYNC_POINTS * segment_blocks;

    *ref_lane = *ref_lane % lanes;

    uint base;
    if (pass != 0) {
        base = lane_blocks - segment_blocks;
    } else {
        if (slice == 0) {
            *ref_lane = lane;
        }
        base = slice * segment_blocks;
    }

    uint ref_area_size = base + offset - 1;
    if (*ref_lane != lane) {
        ref_area_size = min(ref_area_size, base);
    }

    *ref_index = mul_hi(*ref_index, *ref_index);
    *ref_index = ref_area_size - 1 - mul_hi(ref_area_size, *ref_index);

    if (pass != 0 && slice != ARGON2_SYNC_POINTS - 1) {
        *ref_index += (slice + 1) * segment_blocks;
        if (*ref_index >= lane_blocks) {
            *ref_index -= lane_blocks;
        }
    }
}

__device__ __forceinline__ void argon2_core(
        __global struct block_g *memory, __global struct block_g *mem_curr,
        struct block_th *prev, struct block_th *tmp,
        __local struct u64_shuffle_buf *shuffle_buf, uint lanes,
        uint thread, uint pass, uint ref_index, uint ref_lane)
{
    __global struct block_g *mem_ref;
    mem_ref = memory + ref_index * lanes + ref_lane;

#if ARGON2_VERSION == ARGON2_VERSION_10
    load_block_xor(prev, mem_ref, thread);
    move_block(tmp, prev);
#else
    if (pass != 0) {
        load_block(tmp, mem_curr, thread);
        load_block_xor(prev, mem_ref, thread);
        xor_block(tmp, prev);
    } else {
        load_block_xor(prev, mem_ref, thread);
        move_block(tmp, prev);
    }
#endif

    shuffle_block(prev, thread, shuffle_buf);

    xor_block(prev, tmp);

    store_block(mem_curr, prev, thread);
}

__device__ __forceinline__ void next_addresses(struct block_th *addr, struct block_th *tmp,
                    uint thread_input, uint thread,
                    __local struct u64_shuffle_buf *buf)
{
    addr->a = u64_build(0, thread_input);
    addr->b = 0;
    addr->c = 0;
    addr->d = 0;

    shuffle_block(addr, thread, buf);

    addr->a ^= u64_build(0, thread_input);
    move_block(tmp, addr);

    shuffle_block(addr, thread, buf);

    xor_block(addr, tmp);
}

#if ARGON2_TYPE == ARGON2_I || ARGON2_TYPE == ARGON2_ID
struct ref {
    uint ref_lane;
    uint ref_index;
};

/*
 * Refs hierarchy:
 * lanes -> passes -> slices -> blocks
 */
__kernel void argon2_precompute_kernel(
        __local struct u64_shuffle_buf *shuffle_bufs, __global struct ref *refs,
        uint passes, uint lanes, uint segment_blocks)
{
    uint block_id = get_global_id(0) / THREADS_PER_LANE;
    uint warp = get_local_id(0) / THREADS_PER_LANE;
    uint thread = get_local_id(0) % THREADS_PER_LANE;

    __local struct u64_shuffle_buf *shuffle_buf = &shuffle_bufs[warp];

    uint segment_addr_blocks = (segment_blocks + ARGON2_QWORDS_IN_BLOCK - 1)
            / ARGON2_QWORDS_IN_BLOCK;
    uint block = block_id % segment_addr_blocks;
    uint segment = block_id / segment_addr_blocks;

    uint slice, pass, lane;
#if ARGON2_TYPE == ARGON2_ID
    slice = segment % (ARGON2_SYNC_POINTS / 2);
    lane = segment / (ARGON2_SYNC_POINTS / 2);
    pass = 0;
#else
    uint pass_id;

    slice = segment % ARGON2_SYNC_POINTS;
    pass_id = segment / ARGON2_SYNC_POINTS;

    pass = pass_id % passes;
    lane = pass_id / passes;
#endif

    struct block_th addr, tmp;

    uint thread_input;
    switch (thread) {
    case 0:
        thread_input = pass;
        break;
    case 1:
        thread_input = lane;
        break;
    case 2:
        thread_input = slice;
        break;
    case 3:
        thread_input = lanes * segment_blocks * ARGON2_SYNC_POINTS;
        break;
    case 4:
        thread_input = passes;
        break;
    case 5:
        thread_input = ARGON2_TYPE;
        break;
    case 6:
        thread_input = block + 1;
        break;
    default:
        thread_input = 0;
        break;
    }

    next_addresses(&addr, &tmp, thread_input, thread, shuffle_buf);

    refs += segment * segment_blocks;

    for (uint i = 0; i < QWORDS_PER_THREAD; i++) {
        uint pos = i * THREADS_PER_LANE + thread;
        uint offset = block * ARGON2_QWORDS_IN_BLOCK + pos;
        if (offset < segment_blocks) {
            ulong v = block_th_get(&addr, i);
            uint ref_index = u64_lo(v);
            uint ref_lane  = u64_hi(v);

            compute_ref_pos(lanes, segment_blocks, pass, lane, slice, offset,
                            &ref_lane, &ref_index);

            refs[offset].ref_index = ref_index;
            refs[offset].ref_lane  = ref_lane;
        }
    }
}

__device__ __forceinline__ void argon2_step_precompute(
        __global struct block_g *memory, __global struct block_g *mem_curr,
        struct block_th *prev, struct block_th *tmp,
        __local struct u64_shuffle_buf *shuffle_buf,
        __global const struct ref **refs,
        uint lanes, uint segment_blocks, uint thread,
        uint lane, uint pass, uint slice, uint offset)
{
    uint ref_index, ref_lane;
    bool data_independent;
#if ARGON2_TYPE == ARGON2_I
    data_independent = true;
#elif ARGON2_TYPE == ARGON2_ID
    data_independent = pass == 0 && slice < ARGON2_SYNC_POINTS / 2;
#else
    data_independent = false;
#endif
    if (data_independent) {
        ref_index = (*refs)->ref_index;
        ref_lane = (*refs)->ref_lane;
        (*refs)++;
    } else {
        ulong v = u64_shuffle(prev->a, 0, thread, shuffle_buf);
        ref_index = u64_lo(v);
        ref_lane  = u64_hi(v);

        compute_ref_pos(lanes, segment_blocks, pass, lane, slice, offset,
                        &ref_lane, &ref_index);
    }

    argon2_core(memory, mem_curr, prev, tmp, shuffle_buf, lanes, thread, pass,
                ref_index, ref_lane);
}

__kernel void argon2_kernel_segment_precompute(
        __local struct u64_shuffle_buf *shuffle_bufs,
        __global struct block_g *memory, __global const struct ref *refs,
        uint passes, uint lanes, uint segment_blocks,
        uint pass, uint slice)
{
    uint job_id = get_global_id(1);
    uint lane   = get_global_id(0) / THREADS_PER_LANE;
    uint warp   = (get_local_id(1) * get_local_size(0) + get_local_id(0))
            / THREADS_PER_LANE;
    uint thread = get_local_id(0) % THREADS_PER_LANE;

    __local struct u64_shuffle_buf *shuffle_buf = &shuffle_bufs[warp];

    uint lane_blocks = ARGON2_SYNC_POINTS * segment_blocks;

    /* select job's memory region: */
    memory += (size_t)job_id * lanes * lane_blocks;

    struct block_th prev, tmp;

    __global struct block_g *mem_segment =
            memory + slice * segment_blocks * lanes + lane;
    __global struct block_g *mem_prev, *mem_curr;
    uint start_offset = 0;
    if (pass == 0) {
        if (slice == 0) {
            mem_prev = mem_segment + 1 * lanes;
            mem_curr = mem_segment + 2 * lanes;
            start_offset = 2;
        } else {
            mem_prev = mem_segment - lanes;
            mem_curr = mem_segment;
        }
    } else {
        mem_prev = mem_segment + (slice == 0 ? lane_blocks * lanes : 0) - lanes;
        mem_curr = mem_segment;
    }

    load_block(&prev, mem_prev, thread);

#if ARGON2_TYPE == ARGON2_ID
        if (pass == 0 && slice < ARGON2_SYNC_POINTS / 2) {
            refs += lane * (lane_blocks / 2) + slice * segment_blocks;
            refs += start_offset;
        }
#else
        refs += (lane * passes + pass) * lane_blocks + slice * segment_blocks;
        refs += start_offset;
#endif

    for (uint offset = start_offset; offset < segment_blocks; ++offset) {
        argon2_step_precompute(
                    memory, mem_curr, &prev, &tmp, shuffle_buf, &refs, lanes,
                    segment_blocks, thread, lane, pass, slice, offset);

        mem_curr += lanes;
    }
}

__kernel void argon2_kernel_oneshot_precompute(
        __local struct u64_shuffle_buf *shuffle_bufs,
        __global struct block_g *memory, __global const struct ref *refs,
        uint passes, uint lanes, uint segment_blocks)
{
    uint job_id = get_global_id(1);
    uint lane   = get_global_id(0) / THREADS_PER_LANE;
    uint warp   = get_local_id(1) * lanes + get_local_id(0) / THREADS_PER_LANE;
    uint thread = get_local_id(0) % THREADS_PER_LANE;

    __local struct u64_shuffle_buf *shuffle_buf = &shuffle_bufs[warp];

    uint lane_blocks = ARGON2_SYNC_POINTS * segment_blocks;

    /* select job's memory region: */
    memory += (size_t)job_id * lanes * lane_blocks;

    struct block_th prev, tmp;

    __global struct block_g *mem_lane = memory + lane;
    __global struct block_g *mem_prev = mem_lane + 1 * lanes;
    __global struct block_g *mem_curr = mem_lane + 2 * lanes;

    load_block(&prev, mem_prev, thread);

#if ARGON2_TYPE == ARGON2_ID
    refs += lane * (lane_blocks / 2) + 2;
#else
    refs += lane * passes * lane_blocks + 2;
#endif

    uint skip = 2;
    for (uint pass = 0; pass < passes; ++pass) {
        for (uint slice = 0; slice < ARGON2_SYNC_POINTS; ++slice) {
            for (uint offset = 0; offset < segment_blocks; ++offset) {
                if (skip > 0) {
                    --skip;
                    continue;
                }

                argon2_step_precompute(
                            memory, mem_curr, &prev, &tmp, shuffle_buf, &refs,
                            lanes, segment_blocks, thread,
                            lane, pass, slice, offset);

                mem_curr += lanes;
            }

            barrier(CLK_GLOBAL_MEM_FENCE);
        }

        mem_curr = mem_lane;
    }
}
#endif /* ARGON2_TYPE == ARGON2_I || ARGON2_TYPE == ARGON2_ID */

__device__ __forceinline__ void argon2_step(
        __global struct block_g *memory, __global struct block_g *mem_curr,
        struct block_th *prev, struct block_th *tmp, struct block_th *addr,
        __local struct u64_shuffle_buf *shuffle_buf,
        uint lanes, uint segment_blocks, uint thread, uint *thread_input,
        uint lane, uint pass, uint slice, uint offset)
{
    uint ref_index, ref_lane;
    bool data_independent;
#if ARGON2_TYPE == ARGON2_I
    data_independent = true;
#elif ARGON2_TYPE == ARGON2_ID
    data_independent = pass == 0 && slice < ARGON2_SYNC_POINTS / 2;
#else
    data_independent = false;
#endif
    if (data_independent) {
        uint addr_index = offset % ARGON2_QWORDS_IN_BLOCK;
        if (addr_index == 0) {
            if (thread == 6) {
                ++*thread_input;
            }
            next_addresses(addr, tmp, *thread_input, thread, shuffle_buf);
        }

        uint thr = addr_index % THREADS_PER_LANE;
        uint idx = addr_index / THREADS_PER_LANE;

        ulong v = block_th_get(addr, idx);
        v = u64_shuffle(v, thr, thread, shuffle_buf);
        ref_index = u64_lo(v);
        ref_lane  = u64_hi(v);
    } else {
        ulong v = u64_shuffle(prev->a, 0, thread, shuffle_buf);
        ref_index = u64_lo(v);
        ref_lane  = u64_hi(v);
    }

    compute_ref_pos(lanes, segment_blocks, pass, lane, slice, offset,
                    &ref_lane, &ref_index);

    argon2_core(memory, mem_curr, prev, tmp, shuffle_buf, lanes, thread, pass,
                ref_index, ref_lane);
}

__kernel void argon2_kernel_segment(
        __local struct u64_shuffle_buf *shuffle_bufs,
        __global struct block_g *memory, uint passes, uint lanes,
        uint segment_blocks, uint pass, uint slice)
{
    uint job_id = get_global_id(1);
    uint lane   = get_global_id(0) / THREADS_PER_LANE;
    uint warp   = (get_local_id(1) * get_local_size(0) + get_local_id(0))
            / THREADS_PER_LANE;
    uint thread = get_local_id(0) % THREADS_PER_LANE;

    __local struct u64_shuffle_buf *shuffle_buf = &shuffle_bufs[warp];

    uint lane_blocks = ARGON2_SYNC_POINTS * segment_blocks;

    /* select job's memory region: */
    memory += (size_t)job_id * lanes * lane_blocks;

    struct block_th prev, addr, tmp;
    uint thread_input;

#if ARGON2_TYPE == ARGON2_I || ARGON2_TYPE == ARGON2_ID
    switch (thread) {
    case 0:
        thread_input = pass;
        break;
    case 1:
        thread_input = lane;
        break;
    case 2:
        thread_input = slice;
        break;
    case 3:
        thread_input = lanes * lane_blocks;
        break;
    case 4:
        thread_input = passes;
        break;
    case 5:
        thread_input = ARGON2_TYPE;
        break;
    default:
        thread_input = 0;
        break;
    }

    if (pass == 0 && slice == 0 && segment_blocks > 2) {
        if (thread == 6) {
            ++thread_input;
        }
        next_addresses(&addr, &tmp, thread_input, thread, shuffle_buf);
    }
#endif

    __global struct block_g *mem_segment =
            memory + slice * segment_blocks * lanes + lane;
    __global struct block_g *mem_prev, *mem_curr;
    uint start_offset = 0;
    if (pass == 0) {
        if (slice == 0) {
            mem_prev = mem_segment + 1 * lanes;
            mem_curr = mem_segment + 2 * lanes;
            start_offset = 2;
        } else {
            mem_prev = mem_segment - lanes;
            mem_curr = mem_segment;
        }
    } else {
        mem_prev = mem_segment + (slice == 0 ? lane_blocks * lanes : 0) - lanes;
        mem_curr = mem_segment;
    }

    load_block(&prev, mem_prev, thread);

    for (uint offset = start_offset; offset < segment_blocks; ++offset) {
        argon2_step(memory, mem_curr, &prev, &tmp, &addr, shuffle_buf,
                    lanes, segment_blocks, thread, &thread_input,
                    lane, pass, slice, offset);

        mem_curr += lanes;
    }
}

__kernel void argon2_kernel_oneshot(
        __local struct u64_shuffle_buf *shuffle_bufs,
        __global struct block_g *memory, uint passes, uint lanes,
        uint segment_blocks)
{
    uint job_id = get_global_id(1);
    uint lane   = get_global_id(0) / THREADS_PER_LANE;
    uint warp   = get_local_id(1) * lanes + get_local_id(0) / THREADS_PER_LANE;
    uint thread = get_local_id(0) % THREADS_PER_LANE;

    __local struct u64_shuffle_buf *shuffle_buf = &shuffle_bufs[warp];

    uint lane_blocks = ARGON2_SYNC_POINTS * segment_blocks;

    /* select job's memory region: */
    memory += (size_t)job_id * lanes * lane_blocks;

    struct block_th prev, addr, tmp;
    uint thread_input;

#if ARGON2_TYPE == ARGON2_I || ARGON2_TYPE == ARGON2_ID
    switch (thread) {
    case 1:
        thread_input = lane;
        break;
    case 3:
        thread_input = lanes * lane_blocks;
        break;
    case 4:
        thread_input = passes;
        break;
    case 5:
        thread_input = ARGON2_TYPE;
        break;
    default:
        thread_input = 0;
        break;
    }

    if (segment_blocks > 2) {
        if (thread == 6) {
            ++thread_input;
        }
        next_addresses(&addr, &tmp, thread_input, thread, shuffle_buf);
    }
#endif

    __global struct block_g *mem_lane = memory + lane;
    __global struct block_g *mem_prev = mem_lane + 1 * lanes;
    __global struct block_g *mem_curr = mem_lane + 2 * lanes;

    load_block(&prev, mem_prev, thread);

    uint skip = 2;
    for (uint pass = 0; pass < passes; ++pass) {
        for (uint slice = 0; slice < ARGON2_SYNC_POINTS; ++slice) {
            for (uint offset = 0; offset < segment_blocks; ++offset) {
                if (skip > 0) {
                    --skip;
                    continue;
                }

                argon2_step(memory, mem_curr, &prev, &tmp, &addr, shuffle_buf,
                            lanes, segment_blocks, thread, &thread_input,
                            lane, pass, slice, offset);

                mem_curr += lanes;
            }

            barrier(CLK_GLOBAL_MEM_FENCE);

#if ARGON2_TYPE == ARGON2_I || ARGON2_TYPE == ARGON2_ID
            if (thread == 2) {
                ++thread_input;
            }
            if (thread == 6) {
                thread_input = 0;
            }
#endif
        }
#if ARGON2_TYPE == ARGON2_I
        if (thread == 0) {
            ++thread_input;
        }
        if (thread == 2) {
            thread_input = 0;
        }
#endif
        mem_curr = mem_lane;
    }
}





