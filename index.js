let util = require('util')
let bleno = require('bleno')
let UUID = require('./sugar-uuid')

let ServiceNameCharacteristic = require('./characteristics/service-name')
let DeviceModelCharacteristic = require('./characteristics/devie-model')
let WifiNameCharacteristic = require('./characteristics/wifi-name')
let IpAddressCharacteristic = require('./characteristics/ip-address')
let InputCharacteristic = require('./characteristics/input-notify').InputCharacteristic
let NotifyMassageCharacteristic = require('./characteristics/input-notify').NotifyMassageCharacteristic

let BlenoPrimaryService = bleno.PrimaryService
let BlenoCharacteristic = bleno.Characteristic
let BlenoDescriptor = bleno.Descriptor

console.log('Bleno starting...')


function wifiConfService() {
  wifiConfService.super_.call(this, {
    uuid: UUID.SERVICE_ID,
    characteristics: [
      new ServiceNameCharacteristic(),
      new DeviceModelCharacteristic(),
      new WifiNameCharacteristic(),
      new IpAddressCharacteristic(),
      new InputCharacteristic(),
      new NotifyMassageCharacteristic(),
    ]
  })
}

util.inherits(wifiConfService, BlenoPrimaryService)

bleno.on('stateChange', function(state) {
  console.log('on -> stateChange: ' + state + ', address = ' + bleno.address)

  if (state === 'poweredOn') {
    bleno.startAdvertising('raspberrypi', [ UUID.SERVICE_ID ])
  } else {
    bleno.stopAdvertising()
  }
})

// Linux only events /////////////////
bleno.on('accept', function(clientAddress) {
  console.log('on -> accept, client: ' + clientAddress)
  bleno.updateRssi()
})

bleno.on('disconnect', function(clientAddress) {
  console.log('on -> disconnect, client: ' + clientAddress)
})

bleno.on('rssiUpdate', function(rssi) {
  console.log('on -> rssiUpdate: ' + rssi)
})
//////////////////////////////////////

bleno.on('mtuChange', function(mtu) {
  console.log('on -> mtuChange: ' + mtu)
})

bleno.on('advertisingStart', function(error) {
  console.log('on -> advertisingStart: ' + (error ? 'error ' + error : 'success'))

  if (!error) {
    bleno.setServices([
      new wifiConfService()
    ])
  }
})

bleno.on('advertisingStop', function() {
  console.log('on -> advertisingStop')
})

bleno.on('servicesSet', function(error) {
  console.log('on -> servicesSet: ' + (error ? 'error ' + error : 'success'))
})