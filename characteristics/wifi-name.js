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
  const reg = /GENERAL\.CONNECTION:[\s]*([^\n]*)/
  const regCN =  /GENERAL\.连接:[\s]*([^\n]*)/
  let wifiBuffer = execSync('nmcli dev show wlan0')
  let wifiString = wifiBuffer.toString()
  let match = wifiString.match(reg)
  if (!match) {
    match = wifiString.match(regCN)
  }
  if (!match) return 'Not available'
  return match.length > 1 ? match[1] : ''
}

module.exports = WifiNameCharacteristic