// Is used to generate ffmpeg_ffi.rs via the bindgen util

#include <libavutil/common.h>
#include <libavutil/opt.h>
#include <libavutil/channel_layout.h>
#include <libavutil/frame.h>
#include <libavutil/samplefmt.h>
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libswresample/swresample.h>
