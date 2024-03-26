inspect_asm::alloc_vec3::up_a:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	sub rdx, rax
	cmp rdx, 12
	jb .LBB_2
	lea rdx, [rax + 12]
	mov qword ptr [rcx], rdx
.LBB_3:
	mov ecx, dword ptr [rsi + 8]
	mov dword ptr [rax + 8], ecx
	mov rcx, qword ptr [rsi]
	mov qword ptr [rax], rcx
	pop rbx
	ret
.LBB_2:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	jmp .LBB_3