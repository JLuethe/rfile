// SPDX-License-Identifier: GPL-3.0-only
#include <cstddef>
#include <sys/types.h>
#include "magic.h"
#include <cassert>
int main() { magic_t h=magic_open(MAGIC_NONE);assert(h);assert(magic_version()==MAGIC_VERSION);magic_close(h); }
