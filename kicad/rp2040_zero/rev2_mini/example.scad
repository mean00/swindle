/* [Rendering options] */
// Show placeholder PCB in OpenSCAD preview
show_pcb = false;
// Lid mounting method
lid_model = "cap"; // [cap, inner-fit]
// Conditional rendering
render = "case"; // [all, case, lid]


/* [Dimensions] */
// Height of the PCB mounting stand-offs between the bottom of the case and the PCB
standoff_height = 5;
// PCB thickness
pcb_thickness = 1.6;
// Bottom layer thickness
floor_height = 1.2;
// Case wall thickness
wall_thickness = 1.2;
// Space between the top of the PCB and the top of the case
headroom = 0;

/* [Hidden] */
$fa=$preview ? 10 : 4;
$fs=0.2;
inner_height = floor_height + standoff_height + pcb_thickness + headroom;

module wall (thickness, height) {
    linear_extrude(height, convexity=10) {
        difference() {
            offset(r=thickness)
                children();
            children();
        }
    }
}

module bottom(thickness, height) {
    linear_extrude(height, convexity=3) {
        offset(r=thickness)
            children();
    }
}

module lid(thickness, height, edge) {
    linear_extrude(height, convexity=10) {
        offset(r=thickness)
            children();
    }
    translate([0,0,-edge])
    difference() {
        linear_extrude(edge, convexity=10) {
                offset(r=-0.2)
                children();
        }
        translate([0,0, -0.5])
         linear_extrude(edge+1, convexity=10) {
                offset(r=-1.2)
                children();
        }
    }
}


module box(wall_thick, bottom_layers, height) {
    if (render == "all" || render == "case") {
        translate([0,0, bottom_layers])
            wall(wall_thick, height) children();
        bottom(wall_thick, bottom_layers) children();
    }
    
    if (render == "all" || render == "lid") {
        translate([0, 0, height+bottom_layers+0.1])
        lid(wall_thick, bottom_layers, lid_model == "inner-fit" ? headroom-2.5: bottom_layers) 
            children();
    }
}

module mount(drill, space, height) {
    translate([0,0,height/2])
        difference() {
            cylinder(h=height, r=(space/2), center=true);
            cylinder(h=(height*2), r=(drill/2), center=true);
            
            translate([0, 0, height/2+0.01])
                children();
        }
        
}

module connector(min_x, min_y, max_x, max_y, height) {
    size_x = max_x - min_x;
    size_y = max_y - min_y;
    translate([(min_x + max_x)/2, (min_y + max_y)/2, height/2])
        cube([size_x, size_y, height], center=true);
}

module pcb() {
    thickness = 1.6;

    color("#009900")
    difference() {
        linear_extrude(thickness) {
            polygon(points = [[149.007025,89.508975], [149.268629,89.558579], [149.41387086,89.57128491600001], [149.554695528,89.609023284], [149.68683000000001,89.670632508], [149.80625955600001,89.75425578000001], [149.90934585600002,89.857350444], [149.992969128,89.97678], [150.054586716,90.10891447200001], [150.092325084,90.24973914], [150.105025,90.394975], [150.105025,95.594975], [150.09134899999998,95.751255], [150.050741,95.902788], [149.98443799999998,96.04497], [149.894456,96.17348100000001], [149.78353099999998,96.284406], [149.65501999999998,96.37438800000001], [149.512838,96.440691], [149.361305,96.481299], [149.205025,96.494975], [109.241421,96.458579], [109.100633,96.447501], [108.963302,96.414534], [108.832829,96.360489], [108.71240900000001,96.286698], [108.60502100000001,96.194979], [108.51330200000001,96.087591], [108.43951100000001,95.96717100000001], [108.38546600000001,95.83669800000001], [108.35249900000001,95.69936700000001], [108.341421,95.558579], [108.341421,67.194975]]);
        }
    translate([0, 0, -1])
    linear_extrude(thickness+2) 
        polygon(points = [[108.341421,67.194975], [108.355091,67.038695], [108.39569900000001,66.887162], [108.462002,66.74498], [108.551984,66.616469], [108.662909,66.505535], [108.79142,66.415553], [108.93360200000001,66.349259], [109.08513500000001,66.308651], [109.241421,66.294975], [149.168629,66.294975], [149.324915,66.308651], [149.476448,66.349259], [149.61863,66.415553], [149.747141,66.505535], [149.858066,66.616469], [149.948048,66.74498], [150.014351,66.887162], [150.054959,67.038695], [150.068629,67.194975], [150.068629,77.958579], [150.05755100000002,78.099367], [150.024584,78.236698], [149.970539,78.367171], [149.896748,78.487591], [149.80502900000002,78.594979], [149.697641,78.68669799999999], [149.577221,78.76048899999999], [149.446748,78.814534], [149.309417,78.847501], [149.168629,78.858579], [149.007025,78.840975]]);

    }
}

module case_outline() {
    polygon(points = [[107.341421,66.194975], [151.105025,66.194975], [151.105025,97.494975], [107.341421,97.494975]]);
}

rotate([render == "lid" ? 180 : 0, 0, 0])
scale([1, -1, 1])
translate([-129.22322300000002, -81.844975, 0]) {
    pcb_top = floor_height + standoff_height + pcb_thickness;

    difference() {
        box(wall_thickness, floor_height, inner_height) {
            case_outline();
        }

    }

    if (show_pcb && $preview) {
        translate([0, 0, floor_height + standoff_height])
            pcb();
    }

    if (render == "all" || render == "case") {
    }
}
