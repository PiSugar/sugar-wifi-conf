let util = require('util')
let bleno = require('bleno')
let UUID = require('../sugar-uuid')
const os = require('os')

let BlenoCharacteristic = bleno.Characteristic

let IpAddressCharacteristic = function() {
  IpAddressCharacteristic.super_.call(this, {
    uuid: UUID.IP_ADDRESS,
    properties: ['read', 'notify']
  })
}

util.inherits(IpAddressCharacteristic, BlenoCharacteristic)

IpAddressCharacteristic.prototype.onReadRequest = function(offset, callback) {
  let result = this.RESULT_SUCCESS
  let data = new Buffer(getIPAddress())

  if (offset > data.length) {
    result = this.RESULT_INVALID_OFFSET
    data = null
  } else {
    data = data.slice(offset)
  }
  callback(result, data)
}

IpAddressCharacteristic.prototype.onSubscribe = function(maxValueSize, updateValueCallback) {
  console.log('IpAddressCharacteristic subscribe')
  updateValueCallback(new Buffer(getIPAddress()))
  this.changeInterval = setInterval(function() {
    let ip = getIPAddress()
    let data = new Buffer(ip)
    console.log('IpAddressCharacteristic update value: ' + ip)
    updateValueCallback(data)
  }.bind(this), 5000)
}

IpAddressCharacteristic.prototype.onUnsubscribe = function() {
  console.log('IpAddressCharacteristic unsubscribe')
  if (this.changeInterval) {
    clearInterval(this.changeInterval)
    this.changeInterval = null
  }
}

IpAddressCharacteristic.prototype.onNotify = function() {
  console.log('IpAddressCharacteristic on notify')
}

function getIPAddress() {
  let interfaces = os.networkInterfaces();
  for (let index in interfaces) {
    let iface = interfaces[index];
    for (let i = 0; i < iface.length; i++) {
      let alias = iface[i];
      // console.log(alias)
      if (alias.family === 'IPv4' && alias.address !== '127.0.0.1' && !alias.internal) {
        return alias.address;
      }
    }
  }
}

module.exports = IpAddressCharacteristic