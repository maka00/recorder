import sys
import gi

gi.require_version("Gst", "1.0")
from gi.repository import Gst, GLib

Gst.init(None)

pipeline = Gst.parse_launch(
    f"uridecodebin uri=file://{sys.argv[1]} \
! videoconvert \
! v4l2sink device={sys.argv[2]} sync=true"
)

bus = pipeline.get_bus()
bus.add_signal_watch()


def on_segment_done(bus, msg):
    pipeline.seek(
        1.0,
        Gst.Format.TIME,
        Gst.SeekFlags.SEGMENT,
        Gst.SeekType.SET,
        0,
        Gst.SeekType.NONE,
        0,
    )
    return True


bus.connect("message::segment-done", on_segment_done)

pipeline.set_state(Gst.State.PLAYING)
pipeline.get_state(Gst.CLOCK_TIME_NONE)
pipeline.seek(
    1.0,
    Gst.Format.TIME,
    Gst.SeekFlags.SEGMENT,
    Gst.SeekType.SET,
    0,
    Gst.SeekType.NONE,
    0,
)

GLib.MainLoop().run()
