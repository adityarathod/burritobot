# Get all locations

```bash
curl 'https://services.chipotle.com/restaurant/v3/restaurant' --compressed -X POST \
 -H 'Content-Type: application/json' \
 -H 'Ocp-Apim-Subscription-Key: b4d9f36380184a3788857063bce25d6a' \
 --data-raw '{"latitude":37.514844400000015,"longitude":-121.91317609999999,"radius":999999999,"restaurantStatuses":["OPEN","LAB"],"conceptIds":["CMG"],"orderBy":"distance","orderByDescending":false,"pageSize":4000,"pageIndex":0,"embeds":{"addressTypes":["MAIN"],"realHours":true,"directions":true,"catering":true,"onlineOrdering":true,"timezone":true,"marketing":true,"chipotlane":true,"sustainability":true,"experience":true}}' > ~/Desktop/data.json
```

```bash
curl 'https://services.chipotle.com/restaurant/v3/restaurant' --compressed -X POST \
      -H 'Content-Type: application/json' \
      -H 'Ocp-Apim-Subscription-Key: b4d9f36380184a3788857063bce25d6a' \
      --data-raw '{"latitude":0,"longitude":0,"radius":999999999,"restaurantStatuses":["OPEN","LAB"],"conceptIds":["CMG"],"orderBy":"distance","orderByDescending":false,"pageSize":4000,"pageIndex":0,"embeds":{"addressTypes":["MAIN"],"realHours":true,"directions":true,"catering":true,"onlineOrdering":true,"timezone":true,"marketing":true,"chipotlane":true,"sustainability":true,"experience":true}}' -s | jq '.'
```

```bash
curl 'https://services.chipotle.com/menuinnovation/v1/restaurants/3065/onlinemenu?channelId=web&includeUnavailableItems=true' \
      -H 'Ocp-Apim-Subscription-Key: b4d9f36380184a3788857063bce25d6a'
```
