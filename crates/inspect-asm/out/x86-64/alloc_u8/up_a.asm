inspect_asm::alloc_u8::up_a:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	cmp qword ptr [rcx + 8], rax
	je .LBB0_0
	mov rdx, rax
	and rdx, -4
	add rdx, 4
	mov qword ptr [rcx], rdx
	mov byte ptr [rax], sil
	pop rbx
	ret
.LBB0_0:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	mov byte ptr [rax], sil
	pop rbx
	ret
