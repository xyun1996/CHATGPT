import React, { useEffect } from "react";

const HotKey: React.FC = () => {
    useEffect(() => {
        const handleKeyDown = (event: React.KeyboardEvent) => {
            if (event.ctrlKey && event.code == 'KeyC') {
                console.log("CTRL+C has been pressed!");
            }
        }

        const handleNativeKeyDown = (event: KeyboardEvent) => handleKeyDown(event as any);

        window.addEventListener('keydown', handleNativeKeyDown);

        return () => {
            window.removeEventListener('keydown', handleNativeKeyDown);
        }
    }, []);

    return (
        <div>Press Ctrl+C</div>
    )
}
export default HotKey;