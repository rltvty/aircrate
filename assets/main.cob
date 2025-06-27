#scenes
"main_scene"
    AbsoluteNode{left:40% flex_direction:Column}
    "cell"
        Animated<BackgroundColor>{
            idle:#FF0000
            hover:Hsla{ hue:120 saturation:1.0 lightness:0.50 alpha:1.0 }
            press:Hsla{ hue:240 saturation:1.0 lightness:0.50 alpha:1.0 }
        }
        NodeShadow{color:#FF0000 spread_radius:10px blur_radius:5px}
        "text"
            TextLine{text:"Hello, World!, I am writing using cobweb "}


"number_text"
    "cell"
        "text"
            TextLine{text:"placeholder"}
            TextLineColor(Hsla{hue:45 saturation:1.0 lightness:0.5 alpha:1.0})


"exit_button"
    TextLine{text:"Exit"}
"despawn_button"
    TextLine{text:"Despawn"}
"respawn_button"
    TextLine{text:"Respawn"}
