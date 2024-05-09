import React, { useEffect, useState } from 'react';
import 'bootstrap/dist/css/bootstrap.min.css';
import './App.css';
import {
  Spinner,
  CardBody,
  CardTitle,
  Card,
  CardSubtitle,
  Table,
} from "reactstrap";
import { ChainTotalSupplyNumbers } from './types';
// 5 seconds
const UPDATE_TIME = 5000;

const BACKEND_PORT = 9000;
export const SERVER_URL =
  "https://" + window.location.hostname + ":" + BACKEND_PORT + "/";

function App() {
  document.title = "Althea L1 Info"
  const [supplyInfo, setSupplyInfo] = useState<ChainTotalSupplyNumbers | null>(null);

  async function getDistributionInfo() {
    let request_url = SERVER_URL + "supply_info";
    const requestOptions: any = {
      method: "GET",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
    };

    const result = await fetch(request_url, requestOptions);
    const json = await result.json();
    setSupplyInfo(json)
  }


  useEffect(() => {
    getDistributionInfo();
    //eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    const interval = setInterval(() => {
      getDistributionInfo();
    }, UPDATE_TIME);
    return () => clearInterval(interval);
    //eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (supplyInfo == null || typeof (supplyInfo) === "string") {
    return (
      <div className="App-header" style={{ display: "flex", flexWrap: "wrap" }}>
        <Spinner
          color="primary"
          type="grow"
        >
          Loading...
        </Spinner>
      </div>
    )
  }

  return (
    <div className="App-header" style={{ display: "flex", flexWrap: "wrap" }}>
      <div style={{ padding: 5 }}>
        <Card className="ParametersCard" style={{ borderRadius: 8, padding: 25 }}>
          <CardBody>
            <CardTitle tag="h1">
              Gravity Supply Info
            </CardTitle>
            <div style={{ fontSize: 15 }}>Total Supply: {(supplyInfo.total_supply / 10 ** 24).toFixed(2)}M ALTHEA</div>
            <div style={{ fontSize: 15 }}>Community Pool: {(supplyInfo.community_pool / 10 ** 24).toFixed(2)}M ALTHEA</div>
            <div style={{ fontSize: 15 }}>Liquid (Not Vesting): {(supplyInfo.total_liquid_supply / 10 ** 24).toFixed(2)}M ALTHEA</div>
            <div style={{ fontSize: 15 }}>Liquid (Not Vesting) and staked: {(supplyInfo.total_nonvesting_staked / 10 ** 24).toFixed(2)}M ALTHEA</div>
            <div style={{ fontSize: 15 }}>Unclaimed staking rewards: {(supplyInfo.total_unclaimed_rewards / 10 ** 24).toFixed(2)}M ALTHEA</div>
            <div style={{ fontSize: 15 }}>Unvested: {(supplyInfo.total_vesting / 10 ** 24).toFixed(2)}M ALTHEA</div>
            <div style={{ fontSize: 15 }}>Unvested Staked: {(supplyInfo.total_vesting_staked / 10 ** 24).toFixed(2)}M ALTHEA</div>
            <div style={{ fontSize: 15 }}>Vested: {(supplyInfo.total_vested / 10 ** 24).toFixed(2)}M ALTHEA</div>
          </CardBody>
        </Card>
      </div>
    </div >
  );
}

export default App;