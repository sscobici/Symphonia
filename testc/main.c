#include <stdlib.h>
#include "../symphonia-ffi/symphonia.h"

int main(int argc, char const *argv[])
{
    void *mss = sm_io_media_source_stream_new_file("D:\\Media\\Torrent\\Movies\\A.Million.Miles.Away.2023.DV.HDR.2160p.WEB-DL.H265.Master5.mkv");
    void *format = sm_probe(mss);
    SMPacket* packet = sm_format_next_packet(format);
    if (packet != NULL) {
        printf("Packet was decoded");
    }
    return 0;
}
