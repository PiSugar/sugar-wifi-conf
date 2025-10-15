let util = require('util')
let bleno = require('@abandonware/bleno')
let UUID = require('../sugar-uuid')
const os = require('os')

let BlenoCharacteristic = bleno.Characteristic

function getIPAddress() {
  let interfaces = os.networkInterfaces()
  let addresses = []
  for (let index in interfaces) {
    let iface = interfaces[index]
    for (let i = 0; i < iface.length; i++) {
      let alias = iface[i]
      // console.log(alias)
      if (alias.family === 'IPv4' && alias.address !== '127.0.0.1' && !alias.internal) {
        addresses.push(alias.address)
      }
    }
  }
  return addresses.join(', ')
}

function isString(str){
  return Object.prototype.toString.call(str) === "[object String]"
}

module.exports = new BlenoCharacteristic({
  uuid: UUID.IP_ADDRESS,
  properties: ['notify'],
  onSubscribe: function(maxValueSize, updateValueCallback) {
    console.log('IpAddressCharacteristic subscribe')
    this.ip = getIPAddress()
    updateValueCallback(Buffer.from(this.ip))
    this.changeInterval = setInterval(function() {
      this.ip = getIPAddress()
      let data = Buffer.from(isString(this.ip) ? this.ip : '--')
      console.log('IpAddressCharacteristic update value: ' + this.ip)
      updateValueCallback(data)
    }.bind(this), 5000)
  },
  onUnsubscribe: function() {
    console.log('IpAddressCharacteristic unsubscribe')
    if (this.changeInterval) {
      clearInterval(this.changeInterval)
      this.changeInterval = null
    }
  },
  onNotify: function() {
    console.log('IpAddressCharacteristic on notify')
  }
})