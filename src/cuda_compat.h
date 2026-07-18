#pragma once

#include <stdint.h>

typedef unsigned char uchar;
typedef unsigned short ushort;
typedef unsigned int uint;
typedef unsigned long long ulong;

#define __OPENCL_VERSION__ 120
#define __kernel extern "C" __global__
#define __global
#define __private
#define __local
#define __constant __device__ __constant__

#define get_global_id(dim) ((dim) == 0 ? (blockIdx.x * blockDim.x + threadIdx.x) : (blockIdx.y * blockDim.y + threadIdx.y))
#define get_local_id(dim) ((dim) == 0 ? threadIdx.x : threadIdx.y)
#define get_local_size(dim) ((dim) == 0 ? blockDim.x : blockDim.y)
#define CLK_LOCAL_MEM_FENCE 0
#define CLK_GLOBAL_MEM_FENCE 0
#define barrier(flags) __syncthreads()

__device__ __forceinline__ ulong upsample(uint hi, uint lo) {
    return (static_cast<ulong>(hi) << 32) | static_cast<ulong>(lo);
}

__device__ __forceinline__ uint mul_hi(uint a, uint b) {
    return __umulhi(a, b);
}

__device__ __forceinline__ ulong rotate(ulong value, uint shift) {
    shift &= 63U;
    return shift == 0 ? value : ((value << shift) | (value >> (64U - shift)));
}

