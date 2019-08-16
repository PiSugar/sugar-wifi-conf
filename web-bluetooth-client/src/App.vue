<template>
  <div id="app">
    <img class="logo" src="./assets/logo.svg">
    <div class="panel-container">
      <div v-if="!supportBluetooth" class="connect-panel panel">
        <div class="panel-title">Cannot find web bluetooth.</div>
        <p>Please make sure your device and browser support web bluetooth.</p>
      </div>
      <div v-if="!isConnected && supportBluetooth" class="connect-panel panel">
        <div class="panel-title">Connect your pi ...</div>
        <el-row>
          <el-button @click="connectDevice">Discover</el-button>
        </el-row>
      </div>
      <div v-if="isConnected" class="info-panel panel">
        <div class="panel-title">Device Information</div>
        <div v-for="item in infoList" :key="item.uuid" class="info-group">
          <div class="label">{{item.label}}</div>
          <div class="value">{{item.value}}</div>
        </div>
      </div>
      <div v-if="isConnected" class="wifi-panel panel">
        <div class="panel-title">Wifi Setting</div>
        <el-row>
          <el-input placeholder="Wifi name" v-model="wifi">
            <template slot="prepend">Wifi</template>
          </el-input>
        </el-row>
        <el-row>
          <el-input placeholder="Wifi password" v-model="password" type="password">
            <template slot="prepend">Psw</template>
          </el-input>
        </el-row>
        <el-row>
          <el-input placeholder="Key (Default: pisugar)" v-model="key">
            <template slot="prepend">Key</template>
          </el-input>
        </el-row>
        <el-row>
          <el-button>Confirm</el-button>
        </el-row>
      </div>
      <div v-if="isConnected" class="command-panel panel">
        <div class="panel-title">Custom Commands</div>
        <el-row>
          <div class="button-wrap"><el-button size="small" round>Confirm</el-button></div>
          <div class="button-wrap"><el-button size="small" round>Confirm</el-button></div>
          <div class="button-wrap"><el-button size="small" round>Confirm</el-button></div>
          <div class="button-wrap"><el-button size="small" round>Confirm</el-button></div>
          <div class="button-wrap"><el-button size="small" round>Confirm</el-button></div>
        </el-row>
        <el-row>
          <el-input
            type="textarea"
            placeholder="Output"
            v-model="commandOutput"
            :rows="10"
            :disable="true"></el-input>
        </el-row>
      </div>
    </div>
  </div>
</template>

<script>
import { Loading } from 'element-ui';
import UUID from './sugar-uuid'
for (let i in UUID) {
  UUID[i] = UUID[i].toLowerCase()
}
export default {
  name: 'App',
  components: {

  },
  data () {
    return {
      supportBluetooth: false,
      isConnected: false,
      serverId: '',
      wifi: '',
      password: '',
      key: 'pisugar',
      commandOutput: '',
      characteristics: [],
      infoList : [],
      loading: null
    }
  },
  mounted () {
    this.supportBluetooth = window.navigator.bluetooth
  },
  methods: {
    connectDevice () {
      const that = this
      navigator.bluetooth.requestDevice({
        filters: [{
          services: [UUID.SERVICE_ID]
        }]
      })
        .then(device => {
          that.loading = Loading.service({ fullscreen: true })
          return device.gatt.connect()
        })
        .then(server => {
          that.serverId = server.device.id
          return server.getPrimaryService(UUID.SERVICE_ID)
        })
        .then(service => service.getCharacteristics())
        .then(characteristics => {
          that.characteristics = characteristics
          that.isConnected = true
          that.loading.close()
          console.log(that.characteristics)
        })
        .catch(console.log)
    },
    ab2str (buf) {
      return String.fromCharCode.apply(null, new Uint8Array(buf));
    },
    getCharacteristic (uuid) {
      return this.characteristics.find(i => i.uuid === uuid)
    },
    subscribeCharacteristic (uuid) {
      this.getCharacteristic(uuid).addEventListener('characteristicvaluechanged', event => {
        this.infoList.find(i => i.uuid === uuid).value = this.ab2str(event.target.value.buffer)
      })
      this.getCharacteristic(uuid).stopNotifications()
      this.getCharacteristic(uuid).startNotifications()
    },
    readInfoCharacteristic (uuid) {
      this.getCharacteristic(uuid).readValue()
        .then(i => i.buffer)
        .then(this.ab2str)
        .then(this.updateFunc(uuid))
    },
    updateFunc (uuid) {
      let char = this.infoList.find(i => i.uuid === uuid)
      let char_label = this.infoList.find(i => i.uuid_label === uuid)
      return value => {
        if (char) {
          char.value = value
        } else {
          char_label.label = value
        }
      }
    }
  },
  watch: {
    isConnected (val, oldVal) {
      if (val === true && oldVal === false) {
        console.log('Connected!')
        console.log(this.serverId)
        this.infoList = []
        this.infoList.push({
          uuid: '',
          label: 'Device ID',
          value: this.serverId
        })
        this.infoList.push({
          uuid: UUID.DEVICE_MODEL,
          label: 'Model',
          value: ' '
        })
        this.infoList.push({
          uuid: UUID.WIFI_NAME,
          label: 'Wifi',
          value: ' '
        })
        this.infoList.push({
          uuid: UUID.IP_ADDRESS,
          label: 'IP Address',
          value: ' '
        })
        this.readInfoCharacteristic(UUID.DEVICE_MODEL)
        this.subscribeCharacteristic(UUID.WIFI_NAME)
        this.subscribeCharacteristic(UUID.IP_ADDRESS)
        this.characteristics.filter(i => i.uuid.indexOf(UUID.CUSTOM_INFO_LABEL) >= 0).map(i => {
          this.infoList.push({
            uuid: i.uuid.replace(UUID.CUSTOM_INFO_LABEL, UUID.CUSTOM_INFO),
            uuid_label: i.uuid,
            label: '',
            value: ''
          })
          this.readInfoCharacteristic(i.uuid)
          this.subscribeCharacteristic(i.uuid.replace(UUID.CUSTOM_INFO_LABEL, UUID.CUSTOM_INFO))
        })
      }
    }
  }
}
</script>

<style lang="less">
  *{
    box-sizing: border-box;
  }
  html{
    width: 100%;
    height: 100%;
    background-color: orange;
    padding: 0;
    margin: 0;
  }
  body{
    padding: 10px 16px;
    margin: 0;
    background: linear-gradient(#ffe025, orange);
  }
  .logo{
    width: 160px;
  }
  #app {
    font-family: 'Avenir', Helvetica, Arial, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-align: center;
    color: #2c3e50;
    margin-top: 30px;
  }
  .panel{
    position: relative;
    width: 100%;
    min-height: 50px;
    background-color: #fff;
    margin-top: 20px;
    border-radius: 6px;
    box-shadow: 0 0 5px 2px rgba(230, 157, 42, 0.08);
    padding: 15px 20px;
    text-align: left;
    &:after{
      content: ' ';
      display: block;
      clear: both;
    }
  }
  @media screen and (min-width: 900px) {
    .panel-container {
      display: flex;
      justify-content: space-between;
    }
    .panel{
      margin-right: 10px;
    }
  }
  .connect-panel{
    p{
      color: #999;
    }
  }
  .el-row{
    margin-bottom: 15px;
  }
  .command-panel{
    .button-wrap{
      float: left;
      margin: 0 10px 10px 0;
    }
  }
  .panel-title{
    width: 100%;
    height: 30px;
    text-align: left;
    font-size: 16px;
    margin-bottom: 12px;
    font-weight: bold;
  }
  .info-group{
    position: relative;
    float: left;
    display: block;
    text-align: left;
    width: 48%;
    margin-bottom: 20px;
    margin-right: 1%;
    overflow: hidden;
    height: 45px;
    .label{
      font-size: 14px;
      font-weight: bold;
      color: #758699;
    }
    .value{
      width: 96%;
      font-size: 14px;
      overflow:hidden;
      text-overflow:ellipsis;
      white-space:nowrap;
    }
  }
  .code{
    display: block;
    width: 100%;
    height: 100px;
    background-color: #333333;
    color: #e4edff;
    padding: 6px 10px;
  }
</style>
