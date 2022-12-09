#include <linux/random.h>

const __u64 _RNDGETENTCNT = RNDGETENTCNT;
#undef RNDGETENTCNT
const __u64 RNDGETENTCNT = _RNDGETENTCNT;
