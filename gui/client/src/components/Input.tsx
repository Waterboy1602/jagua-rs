import { useState } from "react";
import axios from "axios";
import { useLocation, useNavigate } from "react-router-dom";

import styles from "../style/Input.module.css";

interface Strip {
    Height: number;
}
interface Shape {
    Type: string;
    Data: number[][];
}

interface Item {
    Demand: number;
    DemandMax: number;
    AllowedOrientations: number[];
    Shape: Shape;
}

const makeJSON = (
    name: string,
    items: Item[],
    selected: boolean[],
    strip: Strip
): string => {
    const jsonObj: { Name?: string; Items?: Item[]; Strip?: Strip } = {
        Name: "",
        Items: [],
        Strip: { Height: 0 },
    };

    jsonObj["Name"] = name;
    jsonObj["Items"] = [];

    for (let i = 0; i < selected.length; i++) {
        if (selected[i]) {
            jsonObj["Items"].push(items[i]);
        }
    }
    jsonObj["Strip"] = strip;

    return JSON.stringify(jsonObj);
};

const Input = () => {
    const navigate = useNavigate();
    const location = useLocation();
    const jsonData = location.state?.jsonData;
    const [items, setItems] = useState<Item[]>(jsonData.Items);
    const [selected, setSelected] = useState<boolean[]>(
        new Array(jsonData.Items.length).fill(true)
    );
    const [stripHeight, setStripHeight] = useState<Strip>(jsonData.Strip);

    const handleSubmit = () => {
        const json: string = makeJSON(
            jsonData.Name,
            items,
            selected,
            stripHeight
        );

        console.log(json);

        axios
            .post("http://localhost:8000/json", { json_str: json })
            .then((response) => {
                console.log(response);
                navigate("/result", { state: response.data });
            })
            .catch((err) => {
                console.error(err);
            });
    };

    const handleCheckboxChange = (index: number) => {
        const newSelected = [...selected];
        newSelected[index] = !newSelected[index];
        setSelected(newSelected);
    };

    const SvgComponent = ({ shape }: { shape: Shape }) => {
        const maxX = Math.max(...shape.Data.map((p) => p[0]));
        const maxY = Math.max(...shape.Data.map((p) => p[1]));
        const points = shape.Data.map((p) => `${p[0]},${p[1]}`).join(" ");

        return (
            <div className={styles.svgContainer}>
                <div>
                    <p>Shape:</p>

                    <div className={styles.shape}>
                        <ul>
                            {shape.Data.map(
                                (point: number[], index: number) => (
                                    <li key={index}>
                                        ({point[0]}, {point[1]})
                                    </li>
                                )
                            )}
                        </ul>
                    </div>
                </div>
                <svg
                    viewBox={`-50 -50 ${maxX + 100} ${maxY + 100}`}
                    preserveAspectRatio="xMidYMid meet"
                >
                    <polyline
                        points={points}
                        fill="none"
                        stroke="black"
                        strokeWidth="2"
                        vectorEffect="non-scaling-stroke"
                    />
                </svg>
            </div>
        );
    };

    const renderItems = (items: Item[], selected: boolean[]) => {
        return items.map((item, index: number) => (
            <div key={`item-${index}`}>
                <div
                    className={styles.item}
                    style={{
                        outline: selected[index]
                            ? "3px solid black"
                            : "1px solid black",
                    }}
                >
                    <div
                        className={styles.checkboxContainer}
                        onClick={() => handleCheckboxChange(index)}
                    >
                        <input
                            className={styles.boolean}
                            type="checkbox"
                            checked={selected[index]}
                            onChange={() => handleCheckboxChange(index)}
                            style={{ marginRight: "10px" }}
                        />

                        <h3>Item {index + 1}</h3>
                    </div>

                    <hr />

                    <div className={styles.itemValue}>
                        <p>Demand:</p>
                        <input
                            className={styles.number}
                            type="number"
                            value={item.Demand}
                            onChange={(e) => {
                                const newItems = [...items];
                                newItems[index].Demand = parseInt(
                                    e.target.value
                                );
                                setItems(newItems);
                            }}
                        />
                    </div>

                    <div className={styles.itemValue}>
                        <p>Max demand:</p>
                        <input
                            className={styles.number}
                            type="number"
                            value={item.DemandMax}
                            onChange={(e) => {
                                const newItems = [...items];
                                newItems[index].DemandMax = parseInt(
                                    e.target.value
                                );
                                setItems(newItems);
                            }}
                        />
                    </div>

                    <div className={styles.itemValue}>
                        <p>Orientations:</p>
                        <div>
                            {item.AllowedOrientations.map(
                                (orientation: number, idx: number) => (
                                    <div
                                        className={styles.degreeSymbolWrapper}
                                        key={`orientation-${index}-${idx}`}
                                    >
                                        <input
                                            className={styles.number}
                                            type="number"
                                            value={orientation}
                                            onChange={(e) => {
                                                const newItems = [...items];
                                                newItems[
                                                    index
                                                ].AllowedOrientations[index] =
                                                    parseInt(e.target.value);
                                                setItems(newItems);
                                            }}
                                        />
                                    </div>
                                )
                            )}
                        </div>
                    </div>

                    <SvgComponent key={`svg-${index}`} shape={item.Shape} />
                </div>
            </div>
        ));
    };

    return (
        <div className={styles.container}>
            <div className={styles.title}>
                <h1>{jsonData.Name}.json</h1>

                <button
                    className={styles.submit}
                    type="submit"
                    onClick={handleSubmit}
                >
                    Calculate
                </button>
            </div>

            <div className={styles.items}>
                <div
                    className={styles.item}
                    style={{
                        border: "3px solid black",
                        boxSizing: "border-box",
                    }}
                >
                    <div
                        className={`${styles.checkboxContainer} ${styles.strip}`}
                    >
                        <h3>{jsonData.Strip ? "Strip" : "Bin"}</h3>
                    </div>
                    <hr />
                    <div>
                        <b>Height: </b>
                        <input
                            className={`${styles.number} ${styles.strip}`}
                            type="number"
                            value={jsonData.Strip.Height}
                            onChange={(e) => {
                                setStripHeight({
                                    Height: parseInt(e.target.value),
                                });
                            }}
                        />
                    </div>
                </div>
                {renderItems(jsonData.Items, selected)}
            </div>
        </div>
    );
};

export default Input;
