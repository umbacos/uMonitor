# Example dashboards

Choose **Open** in uMonitor and select one of these editable designs:

- `landscape.json`: 480 x 320 hardware dashboard. Connect a weather source to fill its weather field.
- `portrait.json`: 320 x 480 clock, CPU/RAM and network dashboard.
- `ticker.json`: scrolling CPU/RAM/network fields with individual units and precision.
- `connected-sources.json`: HTTP JSON, Home Assistant, weather and stock source examples.

The first three use local PC readings without API keys. Available hardware sensors
depend on your PC and installed providers; missing readings show `N/A`. Configure
external addresses and credentials in **Data sources**, then choose **Connect /
refresh**. The connection example contains no passwords or API keys.

Every portable build includes this entire directory, including subdirectories and
any supporting assets. Extract the full ZIP before opening an example.
