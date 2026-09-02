add a new panel below the network activity panel
the new panel is called protocols

it will show:
TCP <bytes>
UDP <bytes>

ie show traffic by layer 4 protocol

how to get packages-to-uid mapping:

adb shell pm list packages -U

returns:
package:com.google.android.youtube uid:10166
package:com.android.simappdialog.auto_generated_rro_product__ uid:10060
package:com.android.externalstorage uid:10098
package:com.android.server.telecom uid:1000
etc

then do sysdump

adb shell dumpsys netstats --uid

See sysdump.txt for example

how interpret sysdump values:

Step 3: How to interpret the valuesHere is how to decode that exact block of data:MetricCode NameMeaningNetwork Typetype=1 or type=01 means the app used Wi-Fi. 0 means it used Mobile Data.App Identityuid=10144The unique Android User ID assigned to that specific app.App Stateset=FOREGROUNDTraffic occurred while the app was open on screen (FOREGROUND) or in the background (BACKGROUND).Time Windowst=1787544000Start Time in Unix Epoch format. You can check what hour this was by running date -r 1787544000 on your Mac terminal.Bytes Downloadedrb=24028Received Bytes (approx. 24 KB). This is the key metric for your byte classification.Bytes Uploadedtb=25282Transmitted Bytes (approx. 25 KB).Packetsrp=75 tp=86Received Packets and Transmitted Packets.

How Android Breaks Down Protocols via TagsBy default, an app's traffic is logged under tag=0x0, which represents its total combined bytes. However, Android uses specific internal framework tags to separate basic transport protocols:tag=0x0: Total traffic (All bytes combined).tag=0xfffffff1: System tag specifically tracking UDP traffic.tag=0xfffffff2: System tag specifically tracking TCP traffic.