# Endless-Stream

A simple python script to stream video from a file to a v4l2loopback device. 
This is usefull for testing without a camera.
Script is based on the example in [stackoverflow](https://stackoverflow.com/questions/53747278/seamless-video-loop-in-gstreamer)
## Usage

To run the application, use the following command:

```bash
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
python3 stream.py <full-path-to-video-file> <device-path>
```

## Setup v4l2loopback
### install v4l2loopback
```bash
sudo apt install v4l2loopback-dkms v4l2loopback-utils
```

### Instal via modprobe:

```bash
task install-v4l2loopback
```
or manually:
```bash
sudo modprobe v4l2loopback exclusive_caps=1 video_nr=10 card_label="v4l2loopback"
```
### Test the installation with:
```bash
v4l2-ctl --list-devices
# should contain something like:
# v4l2loopback (platform:v4l2loopback-010):
#        /dev/video10

gst-launch-1.0 videotestsrc ! videoconvert ! v4l2sink device=/dev/video10
# VLC can now open /dev/video10 and display a test screen
```

## Dependencies
* Gstreamer
```bash
# just to be sure all is there
sudo apt install gstreamer1.0-*
sudo apt install libgirepository1.0-dev gir1.2-gstreamer-1.0 gstreamer1.0-dev libcairo2-dev pkg-config python3-dev
```
* https://github.com/umlaeute/v4l2loopback

### ----------

On a server the user has to be in the video group
```bash
sudo usermod -a -G video $LOGNAME
```

