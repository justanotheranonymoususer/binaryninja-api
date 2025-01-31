#pragma once

#include <binaryninjaapi.h>

// This is a temporary C FFI to interact with WARP.

typedef struct BNWARPFunction BNWARPFunction;

extern "C" {
    char* BNWARPGetBasicBlockGUID(BNBasicBlock* basic_block);

    char* BNWARPGetFunctionGUID(BNFunction* analysis_function);

    BNWARPFunction* BNWARPGetMatchedFunction(BNFunction* analysis_function);

    BNWARPFunction* BNWARPSetMatchedFunction(BNFunction* analysis_function, BNWARPFunction* function);

    BNWARPFunction** BNWARPGetPossibleFunctions(BNPlatform* platform, char* guid, size_t* count);

    BNSymbol* BNWARPGetFunctionSymbol(BNFunction* analysis_function, BNWARPFunction* function);

    BNType* BNWARPGetFunctionType(BNFunction* analysis_function, BNWARPFunction* function);

    void BNWARPFreeFunction(BNWARPFunction* function);

    void BNWARPFreeFunctionList(BNWARPFunction** functions, size_t count);
}
