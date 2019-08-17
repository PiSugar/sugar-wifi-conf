const execSync = require('child_process').execSync
let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')

let BlenoCharacteristic = bleno.Characteristic

let WifiNameCharacteristic = function() {
  WifiNameCharacteristic.super_.call(this, {
    uuid: UUID.WIFI_NAME,
    properties: ['notify']
  })
}

util.inherits(WifiNameCharacteristic, BlenoCharacteristic)

WifiNameCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('WifiNameCharacteristic subscribe')
  this.wifiName = getWifiName()
  updateValueCallback(new Buffer(this.wifiName))
  setTimeout(() => {
    updateValueCallback(new Buffer(this.wifiName))
  }, 2000)
  this.changeInterval = setInterval(function() {
    let newWifiName = getWifiName()
    if (newWifiName === this.wifiName) return
    this.wifiName = newWifiName
    let data = new Buffer(this.wifiName)
    console.log('WifiNameCharacteristic update value: ' + this.wifiName)
    updateValueCallback(data)
  }.bind(this), 5000)
}

WifiNameCharacteristic.prototype.onUnsubscribe = function() {
  console.log('WifiNameCharacteristic unsubscribe')

  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

WifiNameCharacteristic.prototype.onNotify = function() {
  console.log('WifiNameCharacteristic on notify')
}

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

module.exports = WifiNameCharacteristic