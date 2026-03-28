# Compatibility shims for Ruby APIs that existed in RPG Maker's bundled Ruby
# but were renamed or changed in modern Ruby (3.x). Ported from mkxp-z's
# ruby_classic_wrap.rb (CC0, WaywardHeart 2023).

class Hash
  alias_method :index, :key unless method_defined?(:index)
end

class Object
  TRUE  = true  unless const_defined?("TRUE")
  FALSE = false unless const_defined?("FALSE")
  NIL   = nil   unless const_defined?("NIL")

  alias_method :id,   :object_id unless method_defined?(:id)
  alias_method :type, :class     unless method_defined?(:type)
end

class NilClass
  def id
    4
  end
end

class TrueClass
  def id
    2
  end
end

# BasicObject#initialize shim intentionally omitted: redefining BasicObject
# methods inside an embedded MRI 3.2 VM triggers rb_estimate_iv_count on the
# root class before its shape data is fully set up, causing a segfault.
# Ruby 3.x already forwards any arguments through BasicObject.new, so the
# shim is not needed.
