const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('@abandonware/bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic

function getWifiName() {
  const reg = /ESSID:"([^"]*)"/
  let match
  try{
    let wifiBuffer = execSync('iwconfig wlan0')
    let wifiString = wifiBuffer.toString()
    match = wifiString.match(reg)
  } catch (e) {
    console.log(e.toString())
  }
  if (!match) return 'Not available'
  return match.length > 1 ? match[1] : 'Not available'
}

module.exports = new BlenoCharacteristic({
  uuid: UUID.WIFI_NAME,
  properties: ['notify'],
  onSubscribe: function(maxValueSize, updateValueCallback) {
    console.log('WifiNameCharacteristic subscribe')
    this.wifiName = getWifiName()
    updateValueCallback(Buffer.from(this.wifiName))
    this.changeInterval = setInterval(function() {
      this.wifiName = getWifiName()
      let data = Buffer.from(this.wifiName)
      console.log('WifiNameCharacteristic update value: ' + this.wifiName)
      updateValueCallback(data)
    }.bind(this), 5000)
  },
  onUnsubscribe: function() {
    console.log('WifiNameCharacteristic unsubscribe')
  
    if (this.changeInterval) {
      clearInterval(this.changeInterval)
      this.changeInterval = null
    }
  },
  onNotify: function() {
    console.log('WifiNameCharacteristic on notify')
  }
})